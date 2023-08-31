use std::cmp::Ordering;

use ring::digest;
use speedy::Writable;
use bytes::Bytes;
use x509_certificate::{
  algorithm::{EcdsaCurve, KeyAlgorithm},
  signing::{InMemorySigningKeyPair, Sign},
};

use crate::{
  security::{
    access_control::*,
    authentication::{
      authentication_builtin::{
        types::{
          BuiltinHandshakeMessageToken, CertificateAlgorithm, HANDSHAKE_REQUEST_CLASS_ID,
          IDENTITY_TOKEN_CLASS_ID,
        },
        HandshakeInfo,
      },
      *,
    },
    certificate::*,
    config::*,
    Authentication, *,
  },
  security_error,
  structure::guid::GuidPrefix,
  QosPolicies, GUID,
};
use super::{
  types::BuiltinIdentityToken, AuthenticationBuiltin, BuiltinHandshakeState, LocalParticipantInfo,
  RemoteParticipantInfo,
};

// DDS Security spec v1.1
// Section "9.3.1 Configuration" , Table 44

const QOS_IDENTITY_CA_PROPERTY_NAME: &str = "dds.sec.auth.identity_ca";
const QOS_IDENTITY_CERTIFICATE_PROPERTY_NAME: &str = "dds.sec.auth.identity_certificate";
const QOS_PRIVATE_KEY_PROPERTY_NAME: &str = "dds.sec.auth.private_key";
const QOS_PASSWORD_PROPERTY_NAME: &str = "dds.sec.auth.password";

impl Authentication for AuthenticationBuiltin {
  fn validate_local_identity(
    &mut self,
    domain_id: u16,
    participant_qos: &QosPolicies,
    candidate_participant_guid: GUID,
  ) -> SecurityResult<(ValidationOutcome, IdentityHandle, GUID)> {
    // TODO 1: Verify identity certificate from PropertyQosPolicy
    //
    // Steps: (spec Table 52, row 1)
    //
    // Load config files from URI's given in PropertyQosPolicy
    // * identity_ca - This is the authority that verifies the authenticity of
    // remote (and our) permissions documents
    // * private_key - Private half of the key that we use to authenticate
    // our identity to others. Others have similar keys.
    // * password - used for decrypting private key, if it is stored encrypted.
    // * identity_certificate - document that contains our subject_name,
    // public half of our identity authentication public key. This is signed by the
    // identity_ca. The purpose of the signature is that, when we send our identity
    // certificate to other Participants, they can verify (with their copy of the CA
    // certificate) that the given public key and subject name belong together.
    // The Domain Governance document gives permissions to subject names, and
    // this binding confirms that the permissions are applicable to he holder of
    // certain public-private-key pair holders.
    //
    // Validate signature of our identity_certificate with identity_ca.
    // If it does not pass, our identity certificate is useless,
    // because others would not accept it either.
    //
    // (Should also check if the certificate has been revoked.
    // The CA certificate may have a revocation list and/or check an on-line OCSP
    // server.)
    //
    // The returned IdentityHandle must be capable of
    // * reading this participant's public key (from identity_certificate)
    // * performing verify and sign operations with this participant's private key
    // * accessing the participant GUID (candidate or adjusted??)

    //TODO: These loading code snippets are too cut-and-paste. Copied from access
    // control.
    let identity_ca = participant_qos
      .get_property(QOS_IDENTITY_CA_PROPERTY_NAME)
      .and_then(|certificate_uri| {
        read_uri(&certificate_uri).map_err(|conf_err| {
          security_error!(
            "Failed to read the identity CA certificate from {}: {:?}",
            certificate_uri,
            conf_err
          )
        })
      })
      .and_then(|certificate_contents_pem| {
        Certificate::from_pem(certificate_contents_pem).map_err(|e| security_error!("{e:?}"))
      })?;

    let identity_certificate = participant_qos
      .get_property(QOS_IDENTITY_CERTIFICATE_PROPERTY_NAME)
      .and_then(|certificate_uri| {
        read_uri(&certificate_uri).map_err(|conf_err| {
          security_error!(
            "Failed to read the DomainParticipant identity certificate from {}: {:?}",
            certificate_uri,
            conf_err
          )
        })
      })
      .and_then(|certificate_contents_pem| {
        Certificate::from_pem(certificate_contents_pem).map_err(|e| security_error!("{e:?}"))
      })?;

    let private_key = participant_qos
      .get_property(QOS_PRIVATE_KEY_PROPERTY_NAME)
      .and_then(|pem_uri| {
        read_uri(&pem_uri).map_err(|conf_err| {
          security_error!(
            "Failed to read the DomainParticipant identity private key from {}: {:?}",
            pem_uri,
            conf_err
          )
        })
      })
      .and_then(|private_key_pem| {
        PrivateKey::from_pem(private_key_pem).map_err(|e| security_error!("{e:?}"))
      })?;

    // Verify that CA has signed our identity
    identity_ca
      .verify_signed_by_certificate(&identity_certificate)
      .map_err(|_e| {
        security_error!("My own identity certificate does not verify against identity CA.")
      })?;

    // TODO: Check (somehow) that my identity has not been revoked.

    // Compute the new adjusted GUID
    // DDS Security spec v1.1 Section "9.3.3 DDS:Auth:PKI-DH plugin behavior", Table
    // 52
    let subject_name_der = identity_certificate.subject_name_der()?;
    let subject_name_der_hash = digest::digest(&digest::SHA256, &subject_name_der);

    // slice and unwrap will succeed, because input size is static
    // Procedure: Take beginning (8 bytes) from subject name DER hash, convert to
    // big-endian u64, so first byte is MSB. Shift u64 right 1 bit and force first
    // bit to one. Now the first bit is one and the following bits are beginning
    // of the SHA256 digest. Truncate this to 48 bites (6 bytes).
    let bytes_from_subject_name =
      &((u64::from_be_bytes(subject_name_der_hash.as_ref()[0..8].try_into().unwrap()) >> 1)
        | 0x8000_0000_0000_0000u64)
        .to_be_bytes()[0..6];

    let candidate_guid_hash =
      digest::digest(&digest::SHA256, &candidate_participant_guid.to_bytes());

    // slicing will succeed, because digest is longer than 6 bytes
    let prefix_bytes = [&bytes_from_subject_name, &candidate_guid_hash.as_ref()[..6]].concat();

    let adjusted_guid = GUID::new(
      GuidPrefix::new(&prefix_bytes),
      candidate_participant_guid.entity_id,
    );

    // Section "9.3.2.1 DDS:Auth:PKI-DH IdentityToken"
    // Table 45
    //
    // TODO: dig out ".algo" values from identity_certificate and identity_ca
    let identity_token = BuiltinIdentityToken {
      certificate_subject: Some(identity_certificate.subject_name().clone().serialize()),
      certificate_algorithm: Some(CertificateAlgorithm::ECPrime256v1),

      ca_subject: Some(identity_ca.subject_name().clone().serialize()),
      ca_algorithm: Some(CertificateAlgorithm::ECPrime256v1),
    };

    let local_identity_handle = self.get_new_identity_handle();

    let local_participant_info = LocalParticipantInfo {
      identity_handle: local_identity_handle,
      identity_token,
      guid: adjusted_guid,
      public_key: identity_certificate,
      private_key,
    };

    self.local_participant_info = Some(local_participant_info);

    Ok((ValidationOutcome::Ok, local_identity_handle, adjusted_guid))
  }

  fn get_identity_token(&self, handle: IdentityHandle) -> SecurityResult<IdentityToken> {
    let local_info = self.get_local_participant_info()?;

    // Parameter handle needs to correspond to the handle of the local participant
    if handle != local_info.identity_handle {
      return Err(security_error!(
        "The given handle does not correspond to the local identity handle"
      ));
    }

    // TODO: return an identity token with actual content
    Ok(local_info.identity_token.clone().into())
  }

  // Currently only mocked
  fn get_identity_status_token(
    &self,
    handle: IdentityHandle,
  ) -> SecurityResult<IdentityStatusToken> {
    // TODO: actual implementation

    Ok(IdentityStatusToken::dummy())
  }

  // Currently only mocked
  fn set_permissions_credential_and_token(
    &self,
    handle: IdentityHandle,
    permissions_credential_token: PermissionsCredentialToken,
    permissions_token: PermissionsToken,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  // The behavior is specified in
  // DDS Security spec v1.1 Section "9.3.3 DDS:Auth:PKI-DH plugin behavior"
  // Table 52, row "validate_remote_identity"
  fn validate_remote_identity(
    &mut self,
    remote_auth_request_token: Option<AuthRequestMessageToken>,
    local_identity_handle: IdentityHandle,
    remote_identity_token: IdentityToken,
    remote_participant_guidp: GuidPrefix,
  ) -> SecurityResult<(
    ValidationOutcome,
    IdentityHandle,
    Option<AuthRequestMessageToken>,
  )> {
    let local_info = self.get_local_participant_info()?;
    // Make sure local_identity_handle is actually ours
    if local_identity_handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter local_identity_handle is not the correct local handle"
      ));
    }

    //let local_identity_token = self.get_identity_token(local_identity_handle)?;

    if remote_identity_token.class_id() != IDENTITY_TOKEN_CLASS_ID {
      //TODO: We are really supposed to ignore differences is MinorVersion of
      // class_id string
      return Err(security_error!(
        "Remote identity class_id is {:?}",
        remote_identity_token.class_id()
      ));
    }

    // Since built-in authentication does not use AuthRequestMessageToken, we ignore
    // them completely. Always return the token as None.
    let auth_request_token = None;

    // The initial handshake state depends on the lexicographic ordering of the
    // participant GUIDs. Note that the derived Ord trait produces the required
    // lexicographic ordering.
    let (handshake_state, validation_outcome) =
      match local_info.guid.prefix.cmp(&remote_participant_guidp) {
        Ordering::Less => {
          // Our GUID is lower than remote's. We should send the request to remote
          (
            BuiltinHandshakeState::PendingRequestSend,
            ValidationOutcome::PendingHandshakeRequest,
          )
        }
        Ordering::Greater => {
          // Our GUID is higher than remote's. We should wait for the request from remote
          (
            BuiltinHandshakeState::PendingRequestMessage,
            ValidationOutcome::PendingHandshakeMessage,
          )
        }
        Ordering::Equal => {
          // This is an error, comparing with ourself.
          return Err(security_error!("Remote GUID is equal to the local GUID"));
        }
      };

    // Get new identity handle for the remote and associate remote info with it
    let remote_identity_handle = self.get_new_identity_handle();

    let remote_info = RemoteParticipantInfo {
      guid_prefix: remote_participant_guidp,
      identity_token: remote_identity_token,
      handshake: HandshakeInfo {
        state: handshake_state,
        // latest_sent_message: None,
        // my_dh_keys: None,
        // challenge1: None,
        // challenge2: None,
        // shared_secret: None,
      },
    };
    self
      .remote_participant_infos
      .insert(remote_identity_handle, remote_info);

    Ok((
      validation_outcome,
      remote_identity_handle,
      auth_request_token,
    ))
  }

  //
  fn begin_handshake_request(
    &mut self,
    initiator_identity_handle: IdentityHandle, // Local
    replier_identity_handle: IdentityHandle,   // Remote
    serialized_local_participant_data: Vec<u8>,
  ) -> SecurityResult<(ValidationOutcome, HandshakeHandle, HandshakeMessageToken)> {
    // Make sure initiator_identity_handle is actually ours
    let local_info = self.get_local_participant_info()?;
    if initiator_identity_handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter initiator_identity_handle is not the correct local handle"
      ));
    }

    // Make sure we are expecting to send the authentication request message
    let remote_info = self.get_remote_participant_info_mutable(&replier_identity_handle)?;
    if let BuiltinHandshakeState::PendingRequestSend = remote_info.handshake.state {
      // Yes, this is what we expect. No action here.
    } else {
      return Err(security_error!(
        "We are not expecting to send a handshake request. Handshake state: {:?}",
        remote_info.handshake.state
      ));
    }

    let my_id_certificate_text = Bytes::from_static(b"dummy"); // TODO: Where to get this? Access control loads it, not us.
    let my_permissions_doc_text = Bytes::from_static(b"dummy"); // TODO: Where to get this? Access control loads it, not us.
    let pdata_bytes = Bytes::from(serialized_local_participant_data);

    let dsign_algo = Bytes::from_static(b"ECDSA-SHA256"); // TODO: do not hardcode this, get from id cert
    let kagree_algo = Bytes::from_static(b"ECDH+prime256v1-CEUM"); // TODO: do not hardcode this, get from id cert

    // temp structure just to produce hash
    let c_properties: Vec<BinaryProperty> = vec![
      BinaryProperty::with_propagate("c.id", my_id_certificate_text.clone()),
      BinaryProperty::with_propagate("c.perm", my_permissions_doc_text.clone()),
      BinaryProperty::with_propagate("c.pdata", pdata_bytes.clone()),
      BinaryProperty::with_propagate("c.dsign_algo", dsign_algo.clone()),
      BinaryProperty::with_propagate("c.kagree_algo", kagree_algo.clone()),
    ];
    let c_properties_bytes = c_properties.write_to_vec_with_ctx(speedy::Endianness::BigEndian)?;
    let c_properties_hash = digest::digest(&digest::SHA256, &c_properties_bytes);

    // Generate new, random Diffie-Hellman key pair "dh1"
    let (dh1_key_pair, _keypair_pkcs8) =
      InMemorySigningKeyPair::generate_random(KeyAlgorithm::Ecdsa(EcdsaCurve::Secp256r1))?;

    // This is an initiator-generated 256-bit nonce
    let challenge1_bytes = rand::random::<[u8; 32]>();

    let handshake_request_builtin = BuiltinHandshakeMessageToken {
      class_id: HANDSHAKE_REQUEST_CLASS_ID.to_string(),
      c_id: Some(my_id_certificate_text),
      c_perm: Some(my_permissions_doc_text),
      c_pdata: Some(pdata_bytes),
      c_dsign_algo: Some(dsign_algo),
      c_kagree_algo: Some(kagree_algo),
      ocsp_status: None, // Not implemented
      hash_c1: Some(Bytes::copy_from_slice(c_properties_hash.as_ref())),
      dh1: Some(dh1_key_pair.public_key_data()),
      hash_c2: None, // not used in request
      dh2: None,     // not used in request
      challenge1: Some(Bytes::copy_from_slice(challenge1_bytes.as_ref())),
      challenge2: None, // not used in request
      signature: None,  // not used in request
    };

    let handshake_request = HandshakeMessageToken::from(handshake_request_builtin);

    // Change handshake state to pending reply message & save the request token
    remote_info.handshake.state = BuiltinHandshakeState::PendingReplyMessage {
      dh1: dh1_key_pair,
      challenge1: challenge1_bytes,
    };
    // remote_info.handshake.my_dh_keys = Some(dh1_key_pair);
    // remote_info.handshake.latest_sent_message = Some(handshake_request.clone());

    // Create a new handshake handle & map it to remotes identity handle
    let new_handshake_handle = self.get_new_handshake_handle();
    self
      .handshake_to_identity_handle_map
      .insert(new_handshake_handle, replier_identity_handle);

    Ok((
      ValidationOutcome::PendingHandshakeMessage,
      new_handshake_handle,
      handshake_request,
    ))
  }

  // Currently only mocked
  fn begin_handshake_reply(
    &mut self,
    handshake_message_in: HandshakeMessageToken,
    initiator_identity_handle: IdentityHandle, // Remote
    replier_identity_handle: IdentityHandle,   // Local
    serialized_local_participant_data: Vec<u8>,
  ) -> SecurityResult<(ValidationOutcome, HandshakeHandle, HandshakeMessageToken)> {
    // Make sure replier_identity_handle is actually ours
    let local_info = self.get_local_participant_info()?;
    if replier_identity_handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter replier_identity_handle is not the correct local handle"
      ));
    }

    // Make sure we are expecting a authentication request from remote
    let remote_info = self.get_remote_participant_info_mutable(&initiator_identity_handle)?;
    if let BuiltinHandshakeState::PendingRequestMessage = remote_info.handshake.state {
      // Nothing to see here. Carry on.
    } else {
      return Err(security_error!(
        "We are not expecting to receive a handshake request. Handshake state: {:?}",
        remote_info.handshake.state
      ));
    }

    let dh1 = Bytes::default(); // TODO get from handshake_message_in
    let challenge1 = <[u8; 32]>::default(); // TODO get from handshake_message_in
                                            // TODO: Check content of authentication request tokens?

    // TODO: Verify the validity of the IdentityCredential in handshake_message_in

    // TODO: check ocsp_status / determine status of IdentityCredential another way

    // TODO: Verify the remote GUID (hash etc.)

    // TODO: Generate a proper reply token
    let reply_token = HandshakeMessageToken::dummy();

    // Generate new, random Diffie-Hellman key pair "dh2"
    let (dh2, _keypair_pkcs8) =
      InMemorySigningKeyPair::generate_random(KeyAlgorithm::Ecdsa(EcdsaCurve::Secp256r1))?;

    // This is an initiator-generated 256-bit nonce
    let challenge2 = rand::random::<[u8; 32]>();

    // Change handshake state to pending final message & save the reply token
    remote_info.handshake.state = BuiltinHandshakeState::PendingFinalMessage {
      dh1,
      challenge1,
      dh2,
      challenge2,
    };
    //remote_info.handshake.latest_sent_message = Some(reply_token.clone());

    // Create a new handshake handle & map it to remotes identity handle
    let new_handshake_handle = self.get_new_handshake_handle();
    self
      .handshake_to_identity_handle_map
      .insert(new_handshake_handle, initiator_identity_handle);

    Ok((
      ValidationOutcome::PendingHandshakeMessage,
      new_handshake_handle,
      reply_token,
    ))
  }

  fn process_handshake(
    &mut self,
    handshake_message_in: HandshakeMessageToken,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<(ValidationOutcome, Option<HandshakeMessageToken>)> {
    // Check what is the handshake state
    let remote_identity_handle = *self.handshake_handle_to_identity_handle(&handshake_handle)?;
    let remote_info = self.get_remote_participant_info_mutable(&remote_identity_handle)?;

    let mut state = BuiltinHandshakeState::PendingRequestSend; // dummy to leave behind
    std::mem::swap(&mut remote_info.handshake.state, &mut state);

    match state {
      BuiltinHandshakeState::PendingReplyMessage { dh1, challenge1 } => {
        // We are the initiator, and expect a reply.
        // Result is that we produce a MassageToken (i.e. send the final message)
        // and the handshake results (shared secret)

        // TODO: verify the contents of the reply message token

        // TODO: verify the content of authentication request token?

        // TODO: Verify validity of IdentityCredential

        // TODO: verify ocsp_status / status of IdentityCredential

        // TODO: check that challenge1 is equal to the challenge1 sent in request
        // token

        // TODO: verify the digital signature

        // TODO: store the value of property with name “dds.sec.” found within the
        // handshake_message_in

        // TODO: Compute the shared secret

        // TODO: Create proper HandshakeFinalMessageToken
        let final_message_token = HandshakeMessageToken::dummy();

        // Generate new, random Diffie-Hellman key pair "dh2"
        let dh2 = Bytes::default(); // TODO: get from handshake message
        let challenge2 = <[u8; 32]>::default();

        // This is an initiator-generated 256-bit nonce
        let challenge2 = rand::random::<[u8; 32]>();

        // Change handshake state to Completed & save the final message token
        remote_info.handshake.state = BuiltinHandshakeState::CompletedWithFinalMessageSent {
          dh1,
          dh2,
          challenge1,
          challenge2,
        };
        // remote_info.handshake.latest_sent_message =
        // Some(final_message_token.clone()); remote_info.handshake.challenge1 =
        // Some(challenge1); remote_info.handshake.challenge2 =
        // Some(challenge2); remote_info.handshake.shared_secret =
        // Some(shared_secret);

        Ok((ValidationOutcome::OkFinalMessage, Some(final_message_token)))
      }
      BuiltinHandshakeState::PendingFinalMessage {
        dh1,
        dh2,
        challenge1,
        challenge2,
      } => {
        // We are the responder, and expect the final message.
        // Result is that we do not produce a MassageToken, since this was the final
        // message, but we compute the handshake results (shared secret)

        // TODO: verify the contents of the final message token

        // TODO: verify matching of challenge1 and challenge2 to what we sent in the
        // reply token

        // TODO: verify the digital signature

        // TODO: compute the shared secret
        // let shared_secret = SharedSecret::default();
        // let challenge1 = Bytes::default();
        // let challenge2 = Bytes::default();

        // Change handshake state to Completed
        remote_info.handshake.state = BuiltinHandshakeState::CompletedWithFinalMessageReceived {
          dh1,
          dh2,
          challenge1,
          challenge2,
        };

        // remote_info.handshake.challenge1 = Some(challenge1);
        // remote_info.handshake.challenge2 = Some(challenge2);
        // remote_info.handshake.shared_secret = Some(shared_secret);

        Ok((ValidationOutcome::Ok, None))
      }
      other_state => {
        // Unexpected state
        Err(security_error!(
          "Unexpected handshake state: {:?}",
          other_state
        ))
      }
    }
  }

  // This function is called after handshake reaches either the state
  // CompletedWithFinalMessageSent or CompletedWithFinalMessageReceived
  fn get_shared_secret(
    &self,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<SharedSecretHandle> {
    let identity_handle = self.handshake_handle_to_identity_handle(&handshake_handle)?;
    let remote_info = self.get_remote_participant_info(identity_handle)?;

    match &remote_info.handshake.state {
      BuiltinHandshakeState::CompletedWithFinalMessageSent {
        dh1,
        dh2,
        challenge1,
        challenge2,
      } => {
        let challenge1 = *challenge1;
        let challenge2 = *challenge2;
        let shared_secret = SharedSecret(<[u8; 32]>::default()); //dummy TODO
        Ok(SharedSecretHandle {
          challenge1,
          challenge2,
          shared_secret,
        })
      }
      BuiltinHandshakeState::CompletedWithFinalMessageReceived {
        dh1,
        dh2,
        challenge1,
        challenge2,
      } => {
        let challenge1 = *challenge1;
        let challenge2 = *challenge2;
        let shared_secret = SharedSecret(<[u8; 32]>::default()); //dummy TODO

        Ok(SharedSecretHandle {
          challenge1,
          challenge2,
          shared_secret,
        })
      }
      wrong_state => Err(security_error!(
        "get_shared_secret called with wrong state {wrong_state:?}"
      )),
    }
  }

  // Currently only mocked
  fn get_authenticated_peer_credential_token(
    &self,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<AuthenticatedPeerCredentialToken> {
    // TODO: actual implementation

    Ok(AuthenticatedPeerCredentialToken::dummy())
  }

  fn set_listener(&self) -> SecurityResult<()> {
    todo!();
  }
}
