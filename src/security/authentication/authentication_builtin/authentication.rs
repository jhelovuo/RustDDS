use std::cmp::Ordering;

use speedy::Writable;
use bytes::{Bytes, BytesMut};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use ring::{agreement, signature};

//use x509_certificate::{
//algorithm::{EcdsaCurve, KeyAlgorithm},
//signing::{InMemorySigningKeyPair, Sign},
//};
use crate::{
  security::{
    access_control::{
      //access_control_builtin::s_mime_config_parser::SignedDocument,
      access_control_builtin::types::BuiltinPermissionsCredentialToken,
      *,
    },
    authentication::{
      authentication_builtin::{
        types::{
          BuiltinHandshakeMessageToken, CertificateAlgorithm, HANDSHAKE_FINAL_CLASS_ID,
          HANDSHAKE_REPLY_CLASS_ID, HANDSHAKE_REQUEST_CLASS_ID, IDENTITY_TOKEN_CLASS_ID,
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
  types::{BuiltinAuthenticatedPeerCredentialToken, BuiltinIdentityToken},
  AuthenticationBuiltin, BuiltinHandshakeState, LocalParticipantInfo, RemoteParticipantInfo,
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
    _domain_id: u16, //TODO: How this should be used?
    participant_qos: &QosPolicies,
    candidate_participant_guid: GUID,
  ) -> SecurityResult<(ValidationOutcome, IdentityHandle, GUID)> {
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

    // TODO: decrypt a password protected private key
    let _password = participant_qos.get_optional_property(QOS_PASSWORD_PROPERTY_NAME);

    let id_cert_private_key = participant_qos
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
    identity_certificate
      .verify_signed_by_certificate(&identity_ca)
      .map_err(|_e| {
        security_error!("My own identity certificate does not verify against identity CA.")
      })?;

    // TODO: Check (somehow) that my identity has not been revoked.

    // Compute the new adjusted GUID
    // DDS Security spec v1.1 Section "9.3.3 DDS:Auth:PKI-DH plugin behavior", Table
    // 52
    let subject_name_der = identity_certificate.subject_name_der()?;
    let subject_name_der_hash = Sha256::hash(&subject_name_der);

    // slice and unwrap will succeed, because input size is static
    // Procedure: Take beginning (8 bytes) from subject name DER hash, convert to
    // big-endian u64, so first byte is MSB. Shift u64 right 1 bit and force first
    // bit to one. Now the first bit is one and the following bits are beginning
    // of the SHA256 digest. Truncate this to 48 bites (6 bytes).
    let bytes_from_subject_name =
      &((u64::from_be_bytes(subject_name_der_hash.as_ref()[0..8].try_into().unwrap()) >> 1)
        | 0x8000_0000_0000_0000u64)
        .to_be_bytes()[0..6];

    let candidate_guid_hash = Sha256::hash(&candidate_participant_guid.to_bytes());

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
      certificate_algorithm: Some(CertificateAlgorithm::ECPrime256v1), // TODO: hardwired
      ca_subject: Some(identity_ca.subject_name().clone().serialize()),
      ca_algorithm: Some(CertificateAlgorithm::ECPrime256v1), // TODO: hardwired
    };

    let local_identity_handle = self.get_new_identity_handle();

    let local_participant_info = LocalParticipantInfo {
      identity_handle: local_identity_handle,
      identity_token,
      guid: adjusted_guid,
      identity_certificate,
      id_cert_private_key,
      identity_ca,
      signed_permissions_document_xml: Bytes::new(), /* This is to filled in later by
                                                      * initialization calling
                                                      * .set_permissions_credential_and_token() */
      local_permissions_token: None, // We do no have it yet.
    };

    self.local_participant_info = Some(local_participant_info);

    // Generate self-shared secret and insert own data into remote_participant_infos
    // This is done for self-authentication.
    // Note: this is not part of the Security specification.
    let random_bytes1 = self.generate_random_32_bytes()?;
    let random_bytes2 = self.generate_random_32_bytes()?;
    let random_bytes3 = self.generate_random_32_bytes()?;

    let self_remote_info = RemoteParticipantInfo {
      identity_certificate_opt: None,
      signed_permissions_xml_opt: None,
      handshake: HandshakeInfo {
        state: BuiltinHandshakeState::CompletedWithFinalMessageReceived {
          challenge1: Challenge::from(random_bytes1),
          challenge2: Challenge::from(random_bytes2),
          shared_secret: SharedSecret::from(random_bytes3),
        },
      },
    };
    self
      .remote_participant_infos
      .insert(local_identity_handle, self_remote_info);

    // Read the temporary config which determines should handshakes be mocked
    self.mock_handshakes = match participant_qos.get_property("dds.sec.auth.mock_handshakes") {
      Ok(val) => val == *"yes",
      Err(_e) => false,
    };

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
    _handle: IdentityHandle,
  ) -> SecurityResult<IdentityStatusToken> {
    // TODO: actual implementation

    Ok(IdentityStatusToken::dummy())
  }

  fn set_permissions_credential_and_token(
    &mut self,
    handle: IdentityHandle,
    permissions_credential_token: PermissionsCredentialToken,
    permissions_token: PermissionsToken,
  ) -> SecurityResult<()> {
    let local_info = self.get_local_participant_info_mutable()?;
    // Make sure local_identity_handle is actually ours
    if handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter local_identity_handle is not the correct local handle"
      ));
    }

    let builtin_token = BuiltinPermissionsCredentialToken::try_from(permissions_credential_token)?;
    local_info.signed_permissions_document_xml = builtin_token.permissions_document;

    // Why do we store this? What is it used for?
    local_info.local_permissions_token = Some(permissions_token);

    Ok(())
  }

  // The behavior is specified in
  // DDS Security spec v1.1 Section "9.3.3 DDS:Auth:PKI-DH plugin behavior"
  // Table 52, row "validate_remote_identity"
  //
  // The name is quite confusing, because this function does not validate much
  // anything, but it starts the authentication protocol.
  fn validate_remote_identity(
    &mut self,
    _remote_auth_request_token: Option<AuthRequestMessageToken>, // Unused, see below.
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
      // TODO: We are really supposed to ignore differences is MinorVersion of
      // class_id string. But now we require exact match.
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
      //guid_prefix: remote_participant_guidp,
      //identity_token: remote_identity_token,
      identity_certificate_opt: None,   // Not yet available
      signed_permissions_xml_opt: None, // Not yet available
      handshake: HandshakeInfo {
        state: handshake_state,
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
    if self.mock_handshakes {
      return self.begin_handshake_request_mocked(
        initiator_identity_handle,
        replier_identity_handle,
        serialized_local_participant_data,
      );
    }

    // Make sure initiator_identity_handle is actually ours
    let local_info = self.get_local_participant_info()?;
    if initiator_identity_handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter initiator_identity_handle is not the correct local handle"
      ));
    }
    let my_id_certificate_text = Bytes::from(local_info.identity_certificate.to_pem());
    let my_permissions_doc_text = local_info.signed_permissions_document_xml.clone();

    let remote_info = self.get_remote_participant_info(&replier_identity_handle)?;

    // Make sure we are expecting to send the authentication request message
    if let BuiltinHandshakeState::PendingRequestSend = remote_info.handshake.state {
      // Yes, this is what we expect. No action here.
    } else {
      return Err(security_error!(
        "We are not expecting to send a handshake request. Handshake state: {:?}",
        remote_info.handshake.state
      ));
    }

    let pdata_bytes = Bytes::from(serialized_local_participant_data);

    let dsign_algo = Bytes::from_static(b"ECDSA-SHA256"); // TODO: do not hardcode this, get from id cert
    let kagree_algo = Bytes::from_static(b"ECDH+prime256v1-CEUM"); // TODO: do not hardcode this, get from id cert

    // temp structure just to produce hash(C1)
    let c_properties: Vec<BinaryProperty> = vec![
      BinaryProperty::with_propagate("c.id", my_id_certificate_text.clone()),
      BinaryProperty::with_propagate("c.perm", my_permissions_doc_text.clone()),
      BinaryProperty::with_propagate("c.pdata", pdata_bytes.clone()),
      BinaryProperty::with_propagate("c.dsign_algo", dsign_algo.clone()),
      BinaryProperty::with_propagate("c.kagree_algo", kagree_algo.clone()),
    ];
    let hash_c1 = Sha256::hash(&c_properties.write_to_vec_with_ctx(speedy::Endianness::BigEndian)?);

    // Generate new, random Diffie-Hellman key pair "dh1"
    let dh1 = agreement::EphemeralPrivateKey::generate(
      &agreement::ECDH_P256,
      &self.secure_random_generator,
    )?;

    // This is an initiator-generated 256-bit nonce
    let random_bytes = self.generate_random_32_bytes()?;
    let challenge1 = Challenge::from(random_bytes);

    let handshake_request_builtin = BuiltinHandshakeMessageToken {
      class_id: Bytes::copy_from_slice(HANDSHAKE_REQUEST_CLASS_ID),
      c_id: Some(my_id_certificate_text),
      c_perm: Some(my_permissions_doc_text),
      c_pdata: Some(pdata_bytes),
      c_dsign_algo: Some(dsign_algo),
      c_kagree_algo: Some(kagree_algo),
      ocsp_status: None, // Not implemented
      hash_c1: Some(Bytes::copy_from_slice(hash_c1.as_ref())),
      dh1: Some(Bytes::copy_from_slice(dh1.compute_public_key()?.as_ref())),
      hash_c2: None, // not used in request
      dh2: None,     // not used in request
      challenge1: Some(Bytes::copy_from_slice(challenge1.as_ref())),
      challenge2: None, // not used in request
      signature: None,  // not used in request
    };

    let handshake_request = HandshakeMessageToken::from(handshake_request_builtin);

    // Get a new mutable reference to remote_info
    let remote_info = self.get_remote_participant_info_mutable(&replier_identity_handle)?;

    // Change handshake state to pending reply message & save the request token
    remote_info.handshake.state = BuiltinHandshakeState::PendingReplyMessage {
      dh1,
      challenge1,
      hash_c1,
    };

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
    if self.mock_handshakes {
      return self.begin_handshake_reply_mocked(
        handshake_message_in,
        initiator_identity_handle,
        replier_identity_handle,
        serialized_local_participant_data,
      );
    }

    // Make sure replier_identity_handle is actually ours
    let local_info = self.get_local_participant_info()?;
    if replier_identity_handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter replier_identity_handle is not the correct local handle"
      ));
    }
    let my_id_certificate_text = Bytes::from(local_info.identity_certificate.to_pem());
    let my_permissions_doc_text = local_info.signed_permissions_document_xml.clone();

    // Make sure we are expecting a authentication request from remote
    let remote_info = self.get_remote_participant_info(&initiator_identity_handle)?;
    if let BuiltinHandshakeState::PendingRequestMessage = remote_info.handshake.state {
      // Nothing to see here. Carry on.
    } else {
      return Err(security_error!(
        "We are not expecting to receive a handshake request. Handshake state: {:?}",
        remote_info.handshake.state
      ));
    }

    let request =
      BuiltinHandshakeMessageToken::try_from(handshake_message_in)?.extract_request()?;

    // "Verifies Cert1 with the configured Identity CA"
    // So Cert1 is now `request.c_id`
    let cert1 = Certificate::from_pem(request.c_id.as_ref())?;

    // Verify that 1's identity cert checks out against CA.
    cert1.verify_signed_by_certificate(&local_info.identity_ca)?;

    let pdata_bytes = Bytes::from(serialized_local_participant_data);

    let dsign_algo = Bytes::from_static(b"ECDSA-SHA256"); // TODO: do not hardcode this, get from id cert
    let kagree_algo = Bytes::from_static(b"ECDH+prime256v1-CEUM"); // TODO: do not hardcode this, get from id cert

    // temp structure just to reproduce hash(c1)
    let c_properties: Vec<BinaryProperty> = vec![
      BinaryProperty::with_propagate("c.id", request.c_id.clone()),
      BinaryProperty::with_propagate("c.perm", request.c_perm.clone()),
      BinaryProperty::with_propagate("c.pdata", request.c_pdata.clone()),
      BinaryProperty::with_propagate("c.dsign_algo", request.c_dsign_algo.clone()),
      BinaryProperty::with_propagate("c.kagree_algo", request.c_kagree_algo.clone()),
    ];
    let computed_c1_hash =
      Sha256::hash(&c_properties.write_to_vec_with_ctx(speedy::Endianness::BigEndian)?);

    // Sanity check, received hash(c1) should match what we computed
    if let Some(received_hash_c1) = request.hash_c1 {
      if received_hash_c1 == computed_c1_hash {
        // hashes match, safe to proceed
      } else {
        return Err(security_error!("begin_handshake_reply: hash_c1 mismatch"));
      }
    } else {
      info!("Cannot compare hashes in begin_handshake_reply. Request did not have any.");
    }

    // This is an initiator-generated 256-bit nonce
    let random_bytes = self.generate_random_32_bytes()?;
    let challenge2 = Challenge::from(random_bytes);

    // Generate new, random Diffie-Hellman key pair "dh2"
    let dh2 = agreement::EphemeralPrivateKey::generate(
      &agreement::ECDH_P256,
      &self.secure_random_generator,
    )?;
    let dh2_public_key = Bytes::copy_from_slice(dh2.compute_public_key()?.as_ref());

    // Compute hash(c2)
    let c2_properties: Vec<BinaryProperty> = vec![
      BinaryProperty::with_propagate("c.id", my_id_certificate_text.clone()),
      BinaryProperty::with_propagate("c.perm", my_permissions_doc_text.clone()),
      BinaryProperty::with_propagate("c.pdata", pdata_bytes.clone()),
      BinaryProperty::with_propagate("c.dsign_algo", dsign_algo.clone()),
      BinaryProperty::with_propagate("c.kagree_algo", kagree_algo.clone()),
    ];
    let c2_hash =
      Sha256::hash(&c2_properties.write_to_vec_with_ctx(speedy::Endianness::BigEndian)?);

    // Spec: "Sign(Hash(C2) | Challenge2 | DH2 | Challenge1 | DH1 | Hash(C1)) )"
    let mut cc2 = BytesMut::with_capacity(1024);
    cc2.extend_from_slice(c2_hash.as_ref());
    cc2.extend_from_slice(challenge2.as_ref());
    cc2.extend_from_slice(dh2_public_key.as_ref());
    cc2.extend_from_slice(request.challenge1.as_ref());
    cc2.extend_from_slice(request.dh1.as_ref());
    cc2.extend_from_slice(computed_c1_hash.as_ref());
    let contents_signature = local_info.id_cert_private_key.sign(cc2.as_ref())?;

    let reply_token = BuiltinHandshakeMessageToken {
      class_id: Bytes::copy_from_slice(HANDSHAKE_REPLY_CLASS_ID),
      c_id: Some(my_id_certificate_text),
      c_perm: Some(my_permissions_doc_text),
      c_pdata: Some(pdata_bytes),
      c_dsign_algo: Some(dsign_algo),
      c_kagree_algo: Some(kagree_algo),
      ocsp_status: None, // Not implemented
      hash_c1: Some(Bytes::copy_from_slice(computed_c1_hash.as_ref())), /* version we computed,
                          * not as received */
      dh1: Some(request.dh1.clone()),
      hash_c2: Some(Bytes::copy_from_slice(c2_hash.as_ref())),
      dh2: Some(dh2_public_key),
      challenge1: Some(Bytes::copy_from_slice(request.challenge1.as_ref())),
      challenge2: Some(Bytes::copy_from_slice(challenge2.as_ref())),
      signature: Some(contents_signature),
    };

    // re-borrow as mutabale
    let remote_info = self.get_remote_participant_info_mutable(&initiator_identity_handle)?;

    // Change handshake state to pending final message & save the reply token
    remote_info.handshake.state = BuiltinHandshakeState::PendingFinalMessage {
      hash_c1: computed_c1_hash,
      hash_c2: c2_hash,
      dh1: request.dh1,
      challenge1: request.challenge1,
      dh2,
      challenge2,
      remote_id_certificate: cert1.clone(),
    };

    // Store remote's ID certificate and permissions doc
    remote_info.identity_certificate_opt = Some(cert1);
    remote_info.signed_permissions_xml_opt = Some(request.c_perm);

    // Create a new handshake handle & map it to remotes identity handle
    let new_handshake_handle = self.get_new_handshake_handle();
    self
      .handshake_to_identity_handle_map
      .insert(new_handshake_handle, initiator_identity_handle);

    Ok((
      ValidationOutcome::PendingHandshakeMessage,
      new_handshake_handle,
      reply_token.into(),
    ))
  }

  fn process_handshake(
    &mut self,
    handshake_message_in: HandshakeMessageToken,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<(ValidationOutcome, Option<HandshakeMessageToken>)> {
    if self.mock_handshakes {
      return self.process_handshake_mocked(handshake_message_in, handshake_handle);
    }

    // Check what is the handshake state
    let remote_identity_handle = *self.handshake_handle_to_identity_handle(&handshake_handle)?;
    let remote_info = self.get_remote_participant_info_mutable(&remote_identity_handle)?;

    // This trickery is needed because BuiltinHandshakeState contains
    // key pairs, which cannot be cloned. We just move the "state" out and leave
    // a dummy value behind. At the end of this function we will overwrite the
    // dummy.
    let mut state = BuiltinHandshakeState::PendingRequestSend; // dummy to leave behind
    std::mem::swap(&mut remote_info.handshake.state, &mut state);

    let local_info = self.get_local_participant_info()?;

    match state {
      BuiltinHandshakeState::PendingReplyMessage {
        dh1,
        challenge1,
        hash_c1,
      } => {
        // We are the initiator, and expect a reply.
        // Result is that we produce a MassageToken (i.e. send the final message)
        // and the handshake results (shared secret)
        let reply =
          BuiltinHandshakeMessageToken::try_from(handshake_message_in)?.extract_reply()?;

        // "Verifies Cert2 with the configured Identity CA"
        // So Cert2 is now `request.c_id`
        let cert2 = Certificate::from_pem(reply.c_id.as_ref())?;

        // Verify that 2's identity cert checks out against CA.
        cert2.verify_signed_by_certificate(&local_info.identity_ca)?;

        // TODO: verify ocsp_status / status of IdentityCredential

        if challenge1 != reply.challenge1 {
          return Err(security_error!(
            "Challenge 1 mismatch on authentication reply"
          ));
        }

        if let Some(received_hash_c1) = reply.hash_c1 {
          if hash_c1 != received_hash_c1 {
            return Err(security_error!("Hash C1 mismatch on authentication reply"));
          } else { /* ok */
          }
        } else {
          debug!("Cannot compare hash C1 in process_handshake. Reply did not have any.");
        }

        // Compute hash(C2) from received data.
        let c2_properties: Vec<BinaryProperty> = vec![
          BinaryProperty::with_propagate("c.id", reply.c_id.clone()),
          BinaryProperty::with_propagate("c.perm", reply.c_perm.clone()),
          BinaryProperty::with_propagate("c.pdata", reply.c_pdata.clone()),
          BinaryProperty::with_propagate("c.dsign_algo", reply.c_dsign_algo.clone()),
          BinaryProperty::with_propagate("c.kagree_algo", reply.c_kagree_algo.clone()),
        ];
        let c2_hash_recomputed =
          Sha256::hash(&c2_properties.write_to_vec_with_ctx(speedy::Endianness::BigEndian)?);

        if let Some(received_hash_c2) = reply.hash_c2 {
          if received_hash_c2.as_ref() == c2_hash_recomputed.as_ref() {
            // hashes match, safe to proceed
          } else {
            return Err(security_error!("process_handshake: hash_c2 mismatch"));
          }
        } else {
          debug!("Cannot compare hashes in process_handshake. Reply did not have any.");
        }

        // Reconstruct signed data: C2 = Cert2, Perm2, Pdata2, Dsign_algo2, Kagree_algo2
        // Spec: "Sign(Hash(C2) | Challenge2 | DH2 | Challenge1 | DH1 | Hash(C1)) )"
        //
        // Note: We already verified above that hash_c1-recomputed vs. hash_c1-stored
        // match and hash_c2 recomputed vs received (if any) match.
        let mut cc2 = BytesMut::with_capacity(1024);
        cc2.extend_from_slice(c2_hash_recomputed.as_ref());
        cc2.extend_from_slice(reply.challenge2.as_ref());
        cc2.extend_from_slice(reply.dh2.as_ref());
        cc2.extend_from_slice(reply.challenge1.as_ref());
        cc2.extend_from_slice(reply.dh1.as_ref());
        cc2.extend_from_slice(hash_c1.as_ref());

        // Verify "C2" contents against reply.signature and 2's public key
        cert2.verify_signed_data_with_algorithm(
          cc2.as_ref(),
          reply.signature,
          // TODO: Hardcoded algorithm
          &signature::ECDSA_P256_SHA256_ASN1,
        )?; // verify ok or exit here

        // Compute the shared secret
        // TODO: Algorithm is hardwired. Should follow "c.kagree_algo" from above.
        let dh1_public = dh1.compute_public_key()?;
        let dh2 = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, reply.dh2.as_ref());
        let shared_secret = agreement::agree_ephemeral(
          dh1,
          &dh2,
          security_error("Shared secret forming failed"), // in case it fails
          |raw_shared_secret| Ok(SharedSecret::from(Sha256::hash(raw_shared_secret))),
        )?;

        // Create signature for final message:
        // Sign( Hash(C1) | Challenge1 | DH1 | Challenge2 | DH2 | Hash(C2) )
        let mut cc_final = BytesMut::with_capacity(1024);
        cc_final.extend_from_slice(hash_c1.as_ref());
        cc_final.extend_from_slice(challenge1.as_ref());
        cc_final.extend_from_slice(dh1_public.as_ref());
        cc_final.extend_from_slice(reply.challenge2.as_ref());
        cc_final.extend_from_slice(reply.dh2.as_ref());
        cc_final.extend_from_slice(c2_hash_recomputed.as_ref());
        let final_contents_signature = local_info.id_cert_private_key.sign(cc_final.as_ref())?;

        // Create HandshakeFinalMessageToken to complete handshake
        // DDS Security spec v1.1 Section  "9.3.2.5.3 HandshakeFinalMessageToken"
        // Table 51 defines contents of the token (message)
        let final_message_token = BuiltinHandshakeMessageToken {
          class_id: Bytes::copy_from_slice(HANDSHAKE_FINAL_CLASS_ID),
          c_id: None,
          c_perm: None,
          c_pdata: None,
          c_dsign_algo: None,
          c_kagree_algo: None,
          ocsp_status: None, // Not implemented
          hash_c1: Some(Bytes::copy_from_slice(hash_c1.as_ref())), // spec says this is optional
          dh1: Some(Bytes::copy_from_slice(dh1_public.as_ref())), // spec says this is optional
          hash_c2: Some(Bytes::copy_from_slice(c2_hash_recomputed.as_ref())), // also optional
          dh2: Some(reply.dh2), // also optional

          // Only the following three parts are mandatory
          challenge1: Some(Bytes::copy_from_slice(reply.challenge1.as_ref())),
          challenge2: Some(Bytes::copy_from_slice(reply.challenge2.as_ref())),
          signature: Some(final_contents_signature),
        };

        // Change handshake state to Completed & save the final message token
        let remote_info = self.get_remote_participant_info_mutable(&remote_identity_handle)?;
        remote_info.handshake.state = BuiltinHandshakeState::CompletedWithFinalMessageSent {
          challenge1,
          challenge2: reply.challenge2,
          shared_secret,
        };

        // Store remote's ID certificate and permissions doc
        remote_info.identity_certificate_opt = Some(cert2);
        remote_info.signed_permissions_xml_opt = Some(reply.c_perm);

        Ok((
          ValidationOutcome::OkFinalMessage,
          Some(HandshakeMessageToken::from(final_message_token)),
        ))
      }

      BuiltinHandshakeState::PendingFinalMessage {
        hash_c1,
        hash_c2,
        dh1,
        dh2,
        challenge1,
        challenge2,
        remote_id_certificate,
      } => {
        // We are the responder, and expect the final message.
        // Result is that we do not produce a MassageToken, since this was the final
        // message, but we compute the handshake results (shared secret)
        let final_token =
          BuiltinHandshakeMessageToken::try_from(handshake_message_in)?.extract_final()?;

        // This is a sanity check
        if let Some(received_hash_c1) = final_token.hash_c1 {
          if hash_c1 != received_hash_c1 {
            return Err(security_error!(
              "Hash C1 mismatch on authentication final receive"
            ));
          }
        }

        // This is a sanity check 2
        if let Some(received_hash_c2) = final_token.hash_c2 {
          if hash_c2 != received_hash_c2 {
            return Err(security_error!(
              "Hash C2 mismatch on authentication final receive"
            ));
          }
        }

        // sanity check
        if dh1 != final_token.dh1 {
          return Err(security_error!(
            "Diffie-Hellman parameter DH1 mismatch on authentication final receive"
          ));
        }

        // sanity check
        if dh2.compute_public_key()?.as_ref() != final_token.dh2.as_ref() {
          return Err(security_error!(
            "Diffie-Hellman parameter DH2 mismatch on authentication final receive"
          ));
        }

        // "The operation shall check that the challenge1 and challenge2 match the ones
        // that were sent on the HandshakeReplyMessageToken."
        if challenge1 != final_token.challenge1 {
          return Err(security_error!(
            "process_handshake: Final token challenge1 mismatch"
          ));
        }
        if challenge2 != final_token.challenge2 {
          //
          return Err(security_error!(
            "process_handshake: Final token challenge2 mismatch"
          ));
        }

        // "The operation shall validate the digital signature in the “signature”
        // property, according to the expected contents and algorithm described
        // in 9.3.2.5.3." ....
        // signature for final message:
        // Sign( Hash(C1) | Challenge1 | DH1 | Challenge2 | DH2 | Hash(C2) )
        let dh2_public = dh2.compute_public_key()?;

        let mut cc_final = BytesMut::with_capacity(1024);
        cc_final.extend_from_slice(hash_c1.as_ref());
        cc_final.extend_from_slice(challenge1.as_ref());
        cc_final.extend_from_slice(dh1.as_ref());
        cc_final.extend_from_slice(challenge2.as_ref());
        cc_final.extend_from_slice(dh2_public.as_ref());
        cc_final.extend_from_slice(hash_c2.as_ref());

        // Now we use the remote certificate, which we verified in the previous (request
        // -> reply) step against CA.
        // TODO: Hardcoded algorithm
        remote_id_certificate
          .verify_signed_data_with_algorithm(
            cc_final.as_ref(),
            final_token.signature,
            &signature::ECDSA_P256_SHA256_ASN1,
          )
          .map_err(|e| {
            security_error!("Signature verification failed in process_handshake: {e:?}")
          })?;

        // Compute the shared secret
        // TODO: Algorithm is hardwired. Should follow "c.kagree_algo" from above.
        let dh1 = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, dh1.as_ref());
        let shared_secret = agreement::agree_ephemeral(
          dh2,
          &dh1,
          security_error("Shared secret forming failed"), // in case it fails
          |raw_shared_secret| Ok(SharedSecret::from(Sha256::hash(raw_shared_secret))),
        )?;

        // Change handshake state to Completed
        let remote_info = self.get_remote_participant_info_mutable(&remote_identity_handle)?;
        remote_info.handshake.state = BuiltinHandshakeState::CompletedWithFinalMessageReceived {
          challenge1,
          challenge2,
          shared_secret,
        };

        Ok((ValidationOutcome::Ok, None))
      }
      other_state => Err(security_error!(
        "Unexpected handshake state: {:?}",
        other_state
      )),
    }
  }

  // This function is called after handshake reaches either the state
  // CompletedWithFinalMessageSent or CompletedWithFinalMessageReceived
  fn get_shared_secret(
    &self,
    remote_identity_handle: IdentityHandle,
  ) -> SecurityResult<SharedSecretHandle> {
    let remote_info = self.get_remote_participant_info(&remote_identity_handle)?;

    match &remote_info.handshake.state {
      BuiltinHandshakeState::CompletedWithFinalMessageSent {
        challenge1,
        challenge2,
        shared_secret,
      } => Ok(SharedSecretHandle {
        challenge1: challenge1.clone(),
        challenge2: challenge2.clone(),
        shared_secret: shared_secret.clone(),
      }),
      BuiltinHandshakeState::CompletedWithFinalMessageReceived {
        challenge1,
        challenge2,
        shared_secret,
      } => Ok(SharedSecretHandle {
        challenge1: challenge1.clone(),
        challenge2: challenge2.clone(),
        shared_secret: shared_secret.clone(),
      }),
      wrong_state => Err(security_error!(
        "get_shared_secret called with wrong state {wrong_state:?}"
      )),
    }
  }

  fn get_authenticated_peer_credential_token(
    &self,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<AuthenticatedPeerCredentialToken> {
    if self.mock_handshakes {
      return self.get_authenticated_peer_credential_token_mocked(handshake_handle);
    }

    let identity_handle = self.handshake_handle_to_identity_handle(&handshake_handle)?;
    let remote_info = self.get_remote_participant_info(identity_handle)?;

    let id_cert = remote_info
      .identity_certificate_opt
      .clone()
      .ok_or_else(|| {
        security_error(
          "Remote's identity certificate missing. It should have been stored from authentication \
           handshake messages",
        )
      })?;

    let permissions_doc = remote_info
      .signed_permissions_xml_opt
      .clone()
      .ok_or_else(|| {
        security_error(
          "Remote's permissions document missing. It should have been stored from authentication \
           handshake messages",
        )
      })?;

    let builtin_token = BuiltinAuthenticatedPeerCredentialToken {
      c_id: Bytes::from(id_cert.to_pem()),
      c_perm: permissions_doc,
    };
    Ok(AuthenticatedPeerCredentialToken::from(builtin_token))
  }

  fn set_listener(&self) -> SecurityResult<()> {
    Err(security_error!(
      "set_listener not supported. Use status events in DataReader/DataWriter instead."
    ))
  }
}
