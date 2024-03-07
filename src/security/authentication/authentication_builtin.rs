use std::{
  collections::HashMap,
  fmt::{self, Formatter},
};

use bytes::Bytes;
use openssl::{bn::BigNum, pkey::Private};
use ring::agreement;

use crate::{
  security::{
    access_control::PermissionsToken, certificate, private_key, security_error, SecurityError,
    SecurityResult,
  },
  security_error, GUID,
};
use self::types::{DH_MODP_KAGREE_ALGO_NAME, ECDH_KAGREE_ALGO_NAME};
use super::{
  authentication_builtin::types::BuiltinIdentityToken, Challenge, HandshakeHandle, IdentityHandle,
  /* IdentityToken, */ Sha256, SharedSecret,
};

mod authentication;
pub(in crate::security) mod types;

// States for an ongoing handshake with a remote participant. Used by the plugin
// internally. Note that there is no 'failed' state, since once a handshake has
// started, it doesn't terminate if some step fails. Instead, it just doesn't
// advance to the next step.
#[derive(Debug)]
pub(crate) enum BuiltinHandshakeState {
  PendingRequestSend,    // We need to create & send the handshake request
  PendingRequestMessage, // We are waiting for a handshake request from remote participant
  PendingReplyMessage {
    // We have sent a handshake request and are waiting for a reply
    dh1: DHKeys,           // both public and private keys for dh1
    challenge1: Challenge, // 256-bit nonce
    hash_c1: Sha256,       // To avoid recomputing this on receiving reply
  },

  // We have sent a handshake reply message and are waiting for the
  // final message
  PendingFinalMessage {
    hash_c1: Sha256,
    hash_c2: Sha256,
    dh1_public: Bytes,     // only public part of dh1
    challenge1: Challenge, // 256-bit nonce
    dh2: DHKeys,           // both public and private keys for dh2
    challenge2: Challenge, // 256-bit nonce
    remote_id_certificate: certificate::Certificate,
  },

  // Handshake was completed & we sent the final message. If
  // requested again, we need to resend the message
  CompletedWithFinalMessageSent {
    // Once we have the shared secret, there should be no need
    // for dh1, dh2, or the challenges.
    // The ring library disallows copying of private DH key exchange keys, so
    // both using and storing them woould be difficult.
    challenge1: Challenge, // 256-bit nonce
    challenge2: Challenge, //256-bit nonce
    shared_secret: SharedSecret,
  },

  // Handshake was completed & we received the final
  // message. Nothing to do for us anymore.
  CompletedWithFinalMessageReceived {
    challenge1: Challenge, // 256-bit nonce
    challenge2: Challenge, // 256-bit nonce
    shared_secret: SharedSecret,
  },
}

// This is a mirror of the above states, but with no data carried from
// one state to another. This is for use in secure Discovery.
// TODO: Refactor (how?) to not need to separate types for this.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum DiscHandshakeState {
  PendingRequestSend,
  PendingRequestMessage,
  PendingReplyMessage,
  PendingFinalMessage,
  CompletedWithFinalMessageSent,
  CompletedWithFinalMessageReceived,
}

struct LocalParticipantInfo {
  identity_handle: IdentityHandle,
  identity_token: BuiltinIdentityToken,
  guid: GUID,
  id_cert_private_key: private_key::PrivateKey, // PrivateKey is actually (private,public) key pair
  identity_certificate: certificate::Certificate, // Certificate contains the public key also
  identity_ca: certificate::Certificate,        /* Certification Authority who has signed
                                                 * identity_certificate */
  signed_permissions_document_xml: Bytes, // We do not care about UTF-8:ness anymore
  local_permissions_token: Option<PermissionsToken>,
}

// All things about remote participant that we're interested in
struct RemoteParticipantInfo {
  //identity_token: IdentityToken,
  //guid_prefix: GuidPrefix,
  identity_certificate_opt: Option<certificate::Certificate>, /* Not available at first.
                                                               * Obtained from handshake
                                                               * request/reply message */
  signed_permissions_xml_opt: Option<Bytes>, /* Not available at first. Obtained from handshake
                                              * request/reply message */
  handshake: HandshakeInfo,
}

// TODO: This struct layer is redundant. Remove and replace with
// BuiltinHandshakeState
struct HandshakeInfo {
  state: BuiltinHandshakeState,
}

pub enum DHKeys {
  Modp(openssl::dh::Dh<Private>), // Modular Exponential keys from OpenSSL
  EC(ring::agreement::EphemeralPrivateKey), // Elliptic Curves keys from ring
}

impl DHKeys {
  fn new_modp_keys() -> SecurityResult<Self> {
    let dh_params = openssl::dh::Dh::get_2048_256()?;
    let modp_keys = dh_params.generate_key()?;
    Ok(Self::Modp(modp_keys))
  }

  fn new_ec_keys(secure_rng: &ring::rand::SystemRandom) -> SecurityResult<Self> {
    let ec_keys = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, secure_rng)?;
    Ok(Self::EC(ec_keys))
  }

  fn public_key_bytes(&self) -> SecurityResult<Bytes> {
    let vec = match self {
      DHKeys::Modp(openssl_dh) => openssl_dh.public_key().to_vec(),
      DHKeys::EC(ring_dh) => {
        let ring_pub_key = ring_dh.compute_public_key()?;
        Vec::from(ring_pub_key.as_ref())
      }
    };
    Ok(Bytes::from(vec))
  }

  fn compute_shared_secret(self, remote_dh_public_key: Bytes) -> SecurityResult<SharedSecret> {
    let shared_secret = match self {
      DHKeys::Modp(openssl_dh) => {
        let remote_public = BigNum::from_slice(&remote_dh_public_key)?;
        let secret_key = openssl_dh.compute_key(&remote_public)?;
        SharedSecret::from(Sha256::hash(&secret_key))
      }
      DHKeys::EC(ring_dh) => {
        let unparsed_remote_dh_public_key =
          agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, remote_dh_public_key);
        agreement::agree_ephemeral(
          ring_dh,
          &unparsed_remote_dh_public_key,
          |raw_shared_secret| SharedSecret::from(Sha256::hash(raw_shared_secret)),
        )?
      }
    };
    Ok(shared_secret)
  }

  fn kagree_algo_name_str(&self) -> &'static str {
    match self {
      DHKeys::Modp(_) => DH_MODP_KAGREE_ALGO_NAME,
      DHKeys::EC(_) => ECDH_KAGREE_ALGO_NAME,
    }
  }
}

// Implement Debug manually because openssl::dh::Dh<Private> does not implement
// it
impl std::fmt::Debug for DHKeys {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      DHKeys::Modp(_dh) => f.debug_struct("Dh<Private>: TODO Debug print").finish(),
      DHKeys::EC(dh) => dh.fmt(f),
    }
  }
}

// A struct implementing the builtin Authentication plugin
// See sections 8.3 and 9.3 of the Security specification (v. 1.1)
pub struct AuthenticationBuiltin {
  local_participant_info: Option<LocalParticipantInfo>,
  remote_participant_infos: HashMap<IdentityHandle, RemoteParticipantInfo>,
  // handshake_to_identity_handle maps handshake handles to identity handles.
  handshake_to_identity_handle_map: HashMap<HandshakeHandle, IdentityHandle>,

  next_identity_handle: IdentityHandle,
  next_handshake_handle: HandshakeHandle,

  // Our own cryptographic pseudo-random number generator
  // From ring documentation (https://docs.rs/ring/latest/ring/rand/index.html):
  // "An application should create a single SystemRandom and then use it for all randomness
  // generation"
  secure_random_generator: ring::rand::SystemRandom,
}

impl AuthenticationBuiltin {
  pub fn new() -> Self {
    Self {
      local_participant_info: None, // No info yet
      remote_participant_infos: HashMap::new(),
      handshake_to_identity_handle_map: HashMap::new(),
      next_identity_handle: 0,
      next_handshake_handle: 0,
      secure_random_generator: ring::rand::SystemRandom::new(),
    }
  }

  fn get_new_identity_handle(&mut self) -> IdentityHandle {
    let new_handle = self.next_identity_handle;
    self.next_identity_handle += 1;
    new_handle
  }

  fn get_new_handshake_handle(&mut self) -> HandshakeHandle {
    let new_handle = self.next_handshake_handle;
    self.next_handshake_handle += 1;
    new_handle
  }

  fn get_local_participant_info(&self) -> SecurityResult<&LocalParticipantInfo> {
    self.local_participant_info.as_ref().ok_or_else(|| {
      security_error!("Local participant info not found. Has the local identity been validated?")
    })
  }

  fn get_local_participant_info_mutable(&mut self) -> SecurityResult<&mut LocalParticipantInfo> {
    self.local_participant_info.as_mut().ok_or_else(|| {
      security_error!("Local participant info not found. Has the local identity been validated?")
    })
  }

  // Returns immutable info
  fn get_remote_participant_info(
    &self,
    identity_handle: &IdentityHandle,
  ) -> SecurityResult<&RemoteParticipantInfo> {
    self
      .remote_participant_infos
      .get(identity_handle)
      .ok_or_else(|| security_error!("Remote participant info not found"))
  }

  // Returns mutable info
  fn get_remote_participant_info_mutable(
    &mut self,
    identity_handle: &IdentityHandle,
  ) -> SecurityResult<&mut RemoteParticipantInfo> {
    self
      .remote_participant_infos
      .get_mut(identity_handle)
      .ok_or_else(|| security_error!("Remote participant info not found"))
  }

  fn handshake_handle_to_identity_handle(
    &self,
    hs_handle: &HandshakeHandle,
  ) -> SecurityResult<&IdentityHandle> {
    self
      .handshake_to_identity_handle_map
      .get(hs_handle)
      .ok_or_else(|| security_error!("Identity handle not found with handshake handle"))
  }

  fn generate_random_32_bytes(&self) -> SecurityResult<[u8; 32]> {
    ring::rand::generate::<[u8; 32]>(&self.secure_random_generator)
      .map(|random| random.expose())
      .map_err(|e| security_error(&format!("Failed to generate random bytes: {}", e)))
  }
}
