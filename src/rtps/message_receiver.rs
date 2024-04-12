use std::collections::{btree_map::Entry, BTreeMap};

use enumflags2::BitFlags;
use mio_extras::{channel as mio_channel, channel::TrySendError};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use bytes::Bytes;

use crate::{
  messages::{protocol_version::ProtocolVersion, submessages::submessages::*, vendor_id::VendorId},
  rtps::{reader::Reader, Message, Submessage, SubmessageBody},
  structure::{
    entity::RTPSEntity,
    guid::{EntityId, GuidPrefix, GUID},
    locator::Locator,
    time::Timestamp,
  },
};
#[cfg(feature = "security")]
use crate::{
  create_security_error,
  security::{
    cryptographic::{DecodeOutcome, DecodedSubmessage, EndpointCryptoHandle},
    security_plugins::SecurityPluginsHandle,
    SecurityError, SecurityResult,
  },
};
#[cfg(not(feature = "security"))]
use crate::no_security::SecurityPluginsHandle;
#[cfg(feature = "rtps_proxy")]
use crate::rtps_proxy::{self, ENTITY_IDS_WITH_NO_DIRECT_RTPS_PROXYING};
#[cfg(test)]
use crate::dds::ddsdata::DDSData;
#[cfg(test)]
use crate::structure::sequence_number::SequenceNumber;

const RTPS_MESSAGE_HEADER_SIZE: usize = 20;

// Secure submessage receiving state machine:
//
// [None] ---SecurePrefix--> [Prefix] ---some Submessage--> [SecureSubmessage]
// ---SecurePostfix--> [None]
//
// [None] ---other submessage--> [None]
//
// If the submessage sequence does not follow either of these, we fail and reset
// to [None].
//
#[cfg(feature = "security")]
#[derive(Clone, Eq, PartialEq, Debug)]
enum SecureReceiverState {
  None,
  // SecurePrefix received
  Prefix(Submessage),
  // Submessage after SecurePrefix
  SecureSubmessage(Submessage, Submessage),
}

// Describes allowed endpoints for an unprotected submessage from security
// perspective
#[derive(PartialEq)]
enum AllowedEndpoints {
  WithNoSubmessageProtection,
  #[cfg(feature = "security")]
  WithCryptoHandles(Vec<EndpointCryptoHandle>),
}

enum UnprotectedSubmessage {
  Interpreter(InterpreterSubmessage),
  Writer(WriterSubmessage, AllowedEndpoints),
  Reader(ReaderSubmessage, AllowedEndpoints),
}

#[cfg(not(feature = "security"))]
impl From<Submessage> for UnprotectedSubmessage {
  fn from(submsg: Submessage) -> Self {
    match submsg.body {
      SubmessageBody::Interpreter(msg) => UnprotectedSubmessage::Interpreter(msg),
      SubmessageBody::Writer(msg) => {
        UnprotectedSubmessage::Writer(msg, AllowedEndpoints::WithNoSubmessageProtection)
      }
      SubmessageBody::Reader(msg) => {
        UnprotectedSubmessage::Reader(msg, AllowedEndpoints::WithNoSubmessageProtection)
      }
    }
  }
}

#[cfg(feature = "security")]
impl From<DecodedSubmessage> for UnprotectedSubmessage {
  fn from(decoded: DecodedSubmessage) -> Self {
    match decoded {
      DecodedSubmessage::Interpreter(msg) => UnprotectedSubmessage::Interpreter(msg),
      DecodedSubmessage::Writer(msg, handles) => {
        UnprotectedSubmessage::Writer(msg, AllowedEndpoints::WithCryptoHandles(handles))
      }
      DecodedSubmessage::Reader(msg, handles) => {
        UnprotectedSubmessage::Reader(msg, AllowedEndpoints::WithCryptoHandles(handles))
      }
    }
  }
}

// This is partial receiver state to be sent to Reader or Writer
#[derive(Debug, Clone)]
pub struct MessageReceiverState {
  pub source_guid_prefix: GuidPrefix,
  pub unicast_reply_locator_list: Vec<Locator>,
  pub multicast_reply_locator_list: Vec<Locator>,
  pub source_timestamp: Option<Timestamp>,
}

impl Default for MessageReceiverState {
  fn default() -> Self {
    Self {
      source_guid_prefix: GuidPrefix::default(),
      unicast_reply_locator_list: Vec::default(),
      multicast_reply_locator_list: Vec::default(),
      source_timestamp: Some(Timestamp::INVALID),
    }
  }
}

/// [`MessageReceiver`] is the submessage sequence interpreter described in
/// RTPS spec v2.3 Section 8.3.4 "The RTPS Message Receiver".
/// It calls the message/submessage deserializers to parse the sequence of
/// submessages. Then it processes the instructions in the Interpreter
/// SUbmessages and forwards data in Entity Submessages to the appropriate
/// Entities. (See RTPS spec Section 8.3.7)

pub(crate) struct MessageReceiver {
  pub available_readers: BTreeMap<EntityId, Reader>,
  // GuidPrefix sent in this channel needs to be RTPSMessage source_guid_prefix. Writer needs this
  // to locate RTPSReaderProxy if negative acknack.
  acknack_sender: mio_channel::SyncSender<(GuidPrefix, AckSubmessage)>,
  // We send notification of remote DomainParticipant liveness to Discovery to
  // bypass Reader, DDSCache, DatasampleCache, and DataReader, because these will drop
  // repeated messages with duplicate SequenceNumbers, but Discovery needs to see them.
  spdp_liveness_sender: mio_channel::SyncSender<GuidPrefix>,
  security_plugins: Option<SecurityPluginsHandle>,

  own_guid_prefix: GuidPrefix,
  pub source_version: ProtocolVersion,
  pub source_vendor_id: VendorId,
  pub source_guid_prefix: GuidPrefix,
  pub dest_guid_prefix: GuidPrefix,
  pub unicast_reply_locator_list: Vec<Locator>,
  pub multicast_reply_locator_list: Vec<Locator>,
  pub source_timestamp: Option<Timestamp>,

  submessage_count: usize, // Used in tests and error messages only?
  #[cfg(feature = "security")]
  // For certain topics we have to allow unprotected rtps messages even if the domain is
  // rtps-protected
  must_be_rtps_protection_special_case: bool,

  #[cfg(feature = "rtps_proxy")]
  proxy_rtps_sender: mio_channel::SyncSender<rtps_proxy::RTPSMessage>,
}

impl MessageReceiver {
  pub fn new(
    participant_guid_prefix: GuidPrefix,
    acknack_sender: mio_channel::SyncSender<(GuidPrefix, AckSubmessage)>,
    spdp_liveness_sender: mio_channel::SyncSender<GuidPrefix>,
    security_plugins: Option<SecurityPluginsHandle>,
    #[cfg(feature = "rtps_proxy")] proxy_rtps_sender: mio_channel::SyncSender<
      rtps_proxy::RTPSMessage,
    >,
  ) -> Self {
    Self {
      available_readers: BTreeMap::new(),
      acknack_sender,
      spdp_liveness_sender,
      security_plugins,
      own_guid_prefix: participant_guid_prefix,

      source_version: ProtocolVersion::THIS_IMPLEMENTATION,
      source_vendor_id: VendorId::VENDOR_UNKNOWN,
      source_guid_prefix: GuidPrefix::UNKNOWN,
      dest_guid_prefix: GuidPrefix::UNKNOWN,
      unicast_reply_locator_list: vec![Locator::Invalid],
      multicast_reply_locator_list: vec![Locator::Invalid],
      source_timestamp: None,

      submessage_count: 0,
      #[cfg(feature = "security")]
      must_be_rtps_protection_special_case: true,
      #[cfg(feature = "rtps_proxy")]
      proxy_rtps_sender,
    }
  }

  pub fn reset(&mut self) {
    self.source_version = ProtocolVersion::THIS_IMPLEMENTATION;
    self.source_vendor_id = VendorId::VENDOR_UNKNOWN;
    self.source_guid_prefix = GuidPrefix::UNKNOWN;
    self.dest_guid_prefix = GuidPrefix::UNKNOWN;
    self.unicast_reply_locator_list.clear();
    self.multicast_reply_locator_list.clear();
    self.source_timestamp = None;

    self.submessage_count = 0;
  }

  fn clone_partial_message_receiver_state(&self) -> MessageReceiverState {
    MessageReceiverState {
      source_guid_prefix: self.source_guid_prefix,
      unicast_reply_locator_list: self.unicast_reply_locator_list.clone(),
      multicast_reply_locator_list: self.multicast_reply_locator_list.clone(),
      source_timestamp: self.source_timestamp,
    }
  }

  pub fn add_reader(&mut self, new_reader: Reader) {
    let eid = new_reader.guid().entity_id;
    match self.available_readers.entry(eid) {
      Entry::Occupied(_) => warn!("Already have Reader {:?} - not adding.", eid),
      Entry::Vacant(e) => {
        e.insert(new_reader);
      }
    }
  }

  pub fn remove_reader(&mut self, old_reader_guid: GUID) -> Option<Reader> {
    self.available_readers.remove(&old_reader_guid.entity_id)
  }

  pub fn reader_mut(&mut self, reader_id: EntityId) -> Option<&mut Reader> {
    self.available_readers.get_mut(&reader_id)
  }

  pub fn handle_received_packet(&mut self, msg_bytes: &Bytes) {
    // Check for RTPS ping message. At least RTI implementation sends these.
    // What should we do with them? The spec does not say.
    if msg_bytes.len() < RTPS_MESSAGE_HEADER_SIZE {
      if msg_bytes.len() >= 16
        && msg_bytes[0..4] == b"RTPS"[..]
        && msg_bytes[9..16] == b"DDSPING"[..]
      {
        // TODO: Add some sensible ping message handling here.
        info!("Received RTPS PING. Do not know how to respond.");
        debug!("Data was {:?}", &msg_bytes);
      } else {
        warn!("Message is shorter than RTPS header. Cannot deserialize.");
        debug!("Data was {:?}", &msg_bytes);
      }
      return;
    }

    // call Speedy reader
    // Bytes .clone() is cheap, so no worries
    let rtps_message = match Message::read_from_buffer(msg_bytes) {
      Ok(m) => m,
      Err(speedy_err) => {
        warn!("RTPS deserialize error {:?}", speedy_err);
        debug!("Data was {:?}", msg_bytes);
        return;
      }
    };

    // And process message
    self.handle_parsed_message(rtps_message);
  }

  // This is also called directly from dp_event_loop in case of loopback messages.
  pub fn handle_parsed_message(&mut self, rtps_message: Message) {
    self.reset();
    self.dest_guid_prefix = self.own_guid_prefix;
    self.source_guid_prefix = rtps_message.header.guid_prefix;
    self.source_version = rtps_message.header.protocol_version;
    self.source_vendor_id = rtps_message.header.vendor_id;

    #[cfg(not(feature = "security"))]
    let unprotected_submessages: Vec<UnprotectedSubmessage> = {
      #[cfg(feature = "rtps_proxy")]
      self.proxy_rtps_message(rtps_message.clone());

      rtps_message
        .submessages
        .into_iter()
        .map(UnprotectedSubmessage::from)
        .collect()
    };

    #[cfg(feature = "security")]
    // Decode the possible RTPS and submessage level protections
    // Note that unlike the normal processing of (unprotected) submessages,
    // the functions called here do not alter the state of the MessageReceiver
    let unprotected_submessages = {
      let rtps_decoded_message = match self.decode_rtps_level_protection(rtps_message) {
        Ok((msg, must_be_rtps_protection_special_case)) => {
          self.must_be_rtps_protection_special_case = must_be_rtps_protection_special_case;
          msg
        }
        Err(e) => {
          warn!("Could not decode RTPS-level protection: {e}");
          return;
        }
      };
      self.decode_submessage_level_protection(rtps_decoded_message)
    };

    // Process the submessages
    for submessage in unprotected_submessages {
      self.handle_unprotected_submessage(submessage);
      self.submessage_count += 1;
    }
  }

  #[cfg(feature = "security")]
  fn decode_rtps_level_protection(&self, rtps_message: Message) -> SecurityResult<(Message, bool)> {
    // Decode RTPS-level protection on a message if it is protected

    let dest_guid_prefix = self.own_guid_prefix;
    let source_guid_prefix = rtps_message.header.guid_prefix;

    let must_be_rtps_protection_special_case: bool;

    match &self.security_plugins {
      None => {
        // No plugins, no protection
        must_be_rtps_protection_special_case = false;
        Ok((rtps_message, must_be_rtps_protection_special_case))
      }

      Some(security_plugins_handle) => {
        let security_plugins = security_plugins_handle.get_plugins();

        // If the first submessage is SecureRTPSPrefix, the message has to be decoded
        // using the cryptographic plugin
        if let Some(Submessage {
          body: SubmessageBody::Security(SecuritySubmessage::SecureRTPSPrefix(..)),
          ..
        }) = rtps_message.submessages.first()
        {
          match security_plugins.decode_rtps_message(rtps_message, &source_guid_prefix) {
            Ok(DecodeOutcome::Success(message)) => {
              must_be_rtps_protection_special_case = false; // Message was protected
              Ok((message, must_be_rtps_protection_special_case))
            }
            Ok(DecodeOutcome::KeysNotFound(header_key_id)) => Err(create_security_error!(
              "No matching message decode keys found for the key id {:?} for the remote \
               participant {:?}",
              header_key_id,
              source_guid_prefix
            )),
            Ok(DecodeOutcome::ValidatingReceiverSpecificMACFailed) => Err(create_security_error!(
              "Failed to validate the receiver-specif MAC for the rtps message."
            )),
            Ok(DecodeOutcome::ParticipantCryptoHandleNotFound(guid_prefix)) => {
              Err(create_security_error!(
                "No participant crypto handle found for the participant {:?} for rtps message \
                 decoding.",
                guid_prefix
              ))
            }
            Err(e) => Err(create_security_error!("{e:?}")),
          }
        } else {
          // No SecureRTPSPrefix. Should there be?
          if security_plugins.rtps_not_protected(&dest_guid_prefix) {
            // The domain is not rtps-protected
            must_be_rtps_protection_special_case = false;
          } else {
            // The messages in a rtps-protected domain are expected to start
            // with SecureRTPSPrefix. The only exception is if the
            // message contains only submessages for the following
            // builtin topics: DCPSParticipants,
            // DCPSParticipantStatelessMessage,
            // DCPSParticipantVolatileMessageSecure
            // (8.4.2.4, table 27).
            must_be_rtps_protection_special_case = true;
          }
          Ok((rtps_message, must_be_rtps_protection_special_case))
        }
      }
    }
  }

  #[cfg(feature = "security")]
  fn decode_submessage_level_protection(
    &self,
    rtps_message: Message,
  ) -> Vec<UnprotectedSubmessage> {
    let mut source_guid_prefix = rtps_message.header.guid_prefix;
    let mut dest_guid_prefix = self.own_guid_prefix;

    let mut unprotected_submessages: Vec<UnprotectedSubmessage> = vec![];

    #[cfg(feature = "rtps_proxy")]
    let mut proxied_submessages: Vec<Submessage> = vec![];

    let mut secure_receiver_state = SecureReceiverState::None;

    for new_submessage in rtps_message.submessages.into_iter() {
      #[cfg(feature = "rtps_proxy")]
      let new_submessage_clone = new_submessage.clone();

      if let SecureReceiverState::Prefix(prefix) = secure_receiver_state {
        // If we have a SecurePrefix stored, the new submessage is always stored in
        // secure receiving state
        secure_receiver_state = SecureReceiverState::SecureSubmessage(prefix, new_submessage);
      } else {
        match new_submessage.body {
          SubmessageBody::Interpreter(msg) => {
            // Plain interpreter submessage. Save source/destination if present, since these
            // affect how protected messages need to be decoded
            match msg {
              InterpreterSubmessage::InfoSource(info_src, _flags) => {
                source_guid_prefix = info_src.guid_prefix;
              }
              InterpreterSubmessage::InfoDestination(ref info_dest, _flags) => {
                dest_guid_prefix = if info_dest.guid_prefix != GUID::GUID_UNKNOWN.prefix {
                  self.own_guid_prefix
                } else {
                  info_dest.guid_prefix
                };
              }
              _ => {}
            }
            unprotected_submessages.push(UnprotectedSubmessage::Interpreter(msg));

            #[cfg(feature = "rtps_proxy")]
            proxied_submessages.push(new_submessage_clone);
          }
          SubmessageBody::Writer(msg) => {
            // Plain Writer submessage. Only readers with no submessage protection are
            // allowed no process the message
            unprotected_submessages.push(UnprotectedSubmessage::Writer(
              msg,
              AllowedEndpoints::WithNoSubmessageProtection,
            ));

            #[cfg(feature = "rtps_proxy")]
            proxied_submessages.push(new_submessage_clone);
          }
          SubmessageBody::Reader(msg) => {
            // Plain Reader submessage. Only writers with no submessage protection are
            // allowed no process the message
            unprotected_submessages.push(UnprotectedSubmessage::Reader(
              msg,
              AllowedEndpoints::WithNoSubmessageProtection,
            ));

            #[cfg(feature = "rtps_proxy")]
            proxied_submessages.push(new_submessage_clone);
          }
          SubmessageBody::Security(ref sec_submsg) => {
            // Security submessage
            // Check if we have security enabled
            let sec_plugins_handle = match self.security_plugins.as_ref() {
              Some(handle) => handle,
              None => {
                warn!(
                  "Received a Secure submessage, but Security hasn't been enabled. Discarding the \
                   submessage."
                );
                continue;
              }
            };

            // Check if this could be meant for us
            if !(dest_guid_prefix == self.own_guid_prefix
              || dest_guid_prefix == GuidPrefix::UNKNOWN)
            {
              trace!(
                "Secure submessage is not for this participant. Dropping. dest_guid_prefix={:?} \
                 participant guid={:?}",
                dest_guid_prefix,
                self.own_guid_prefix
              );
              #[cfg(feature = "rtps_proxy")]
              proxied_submessages.push(new_submessage_clone);

              continue;
            }

            // Security submessages should arrive in a (prefix, submsg, postfix) sequence.
            // The variable secure_receiver_state holds the receiving state.
            match sec_submsg {
              SecuritySubmessage::SecurePrefix(_prefix, _prefix_flags) => {
                // Received a SecurePrefix. This should start a new (prefix, submsg, postfix)
                // sequence.
                match secure_receiver_state {
                  SecureReceiverState::None => {
                    secure_receiver_state = SecureReceiverState::Prefix(new_submessage);
                  }
                  SecureReceiverState::Prefix(_existing_prefix) => {
                    warn!(
                      "Received another SecurePrefix while one was already in store. Discarding \
                       the first one."
                    );
                    secure_receiver_state = SecureReceiverState::Prefix(new_submessage);
                  }
                  SecureReceiverState::SecureSubmessage(_prefix, _submsg) => {
                    warn!(
                      "Received SecurePrefix while waiting for SecurePostfix. Keeping only the \
                       new prefix."
                    );
                    secure_receiver_state = SecureReceiverState::Prefix(new_submessage);
                  }
                }
              }
              SecuritySubmessage::SecureBody(_body, _body_flags) => {
                // Received a SecureBody. We should have received a SecurePrefix just before
                // this.
                match secure_receiver_state {
                  SecureReceiverState::None => {
                    warn!("Received a SecureBody without a preceding SecurePrefix. Discarding.");
                  }
                  SecureReceiverState::Prefix(prefix) => {
                    secure_receiver_state =
                      SecureReceiverState::SecureSubmessage(prefix, new_submessage);
                  }
                  SecureReceiverState::SecureSubmessage(_prefix, _submsg) => {
                    warn!(
                      "Received a SecureBody while waiting for a SecurePostfix. Discarding the \
                       secure submessages"
                    );
                    secure_receiver_state = SecureReceiverState::None;
                  }
                }
              }
              SecuritySubmessage::SecurePostfix(postfix, _postfix_flags) => {
                // Received a SecurePostfix. We should have received a SecurePrefix & submsg
                // just before this.
                match secure_receiver_state {
                  SecureReceiverState::None => {
                    warn!(
                      "Received a SecurePostfix without preceding SecurePrefix & submsg. \
                       Discarding."
                    );
                  }
                  SecureReceiverState::Prefix(_prefix) => {
                    warn!("Received a SecurePostfix while waiting a submsg. Discarding.");
                    secure_receiver_state = SecureReceiverState::None;
                  }
                  SecureReceiverState::SecureSubmessage(prefix_submsg, submsg) => {
                    // Now we have the full (prefix, submsg, postfix) sequence.

                    #[cfg(feature = "rtps_proxy")]
                    // Clone the submessages for possible proxying. We'll proxy them if
                    //  1. We can't decode the messages, since they're probably meant for someone
                    //     else
                    //  2. We can decode them, but the resulting submessage is in a topic which
                    //     should not be proxied, or
                    let mut secure_submessages_for_proxying = vec![
                      prefix_submsg.clone(),
                      submsg.clone(),
                      new_submessage.clone(), // The postfix
                    ];

                    // Verify that the prefix submessage is actually a secure prefix
                    if let SubmessageBody::Security(SecuritySubmessage::SecurePrefix(
                      prefix,
                      _flags,
                    )) = prefix_submsg.body
                    {
                      // Attempt to decode the sequence
                      let decode_result = sec_plugins_handle
                        .get_plugins()
                        .decode_submessage((prefix, submsg, postfix.clone()), &source_guid_prefix);

                      // Reset secure receiving state
                      secure_receiver_state = SecureReceiverState::None;

                      #[cfg(feature = "rtps_proxy")]
                      // Whether the secure submessages are proxied depends on the decoding result
                      let proxy_the_secure_submessages: bool;

                      match decode_result {
                        Err(e) => {
                          warn!(
                            "Decoding a secure submessage from participant {source_guid_prefix:?} \
                             failed: {e:?}"
                          );
                          #[cfg(feature = "rtps_proxy")]
                          {
                            proxy_the_secure_submessages = true;
                          }
                        }
                        Ok(DecodeOutcome::KeysNotFound(header_key_id)) => {
                          warn!(
                            "No matching submessage decode keys found for the key id {:?} for the \
                             remote participant {:?}",
                            header_key_id, source_guid_prefix
                          );
                          #[cfg(feature = "rtps_proxy")]
                          {
                            proxy_the_secure_submessages = true;
                          }
                        }
                        Ok(DecodeOutcome::ValidatingReceiverSpecificMACFailed) => {
                          warn!(
                            "No endpoints passed the receiver-specific MAC validation for the \
                             submessage. Remote participant: {source_guid_prefix:?}"
                          );
                          #[cfg(feature = "rtps_proxy")]
                          {
                            proxy_the_secure_submessages = true;
                          }
                        }
                        Ok(DecodeOutcome::ParticipantCryptoHandleNotFound(guid_prefix)) => {
                          warn!(
                            "No participant crypto handle found for the participant {:?} for \
                             submessage decoding.",
                            guid_prefix
                          );
                          #[cfg(feature = "rtps_proxy")]
                          {
                            proxy_the_secure_submessages = true;
                          }
                        }
                        Ok(DecodeOutcome::Success(decoded_msg)) => {
                          // Decoding successful
                          if let DecodedSubmessage::Interpreter(interpreter_submsg) = &decoded_msg {
                            // Decoded an interpreter submessage. Save source/destination if
                            // present, since these affect how protected
                            // messages need to be decoded
                            match interpreter_submsg {
                              InterpreterSubmessage::InfoSource(info_src, _flags) => {
                                source_guid_prefix = info_src.guid_prefix;
                              }
                              InterpreterSubmessage::InfoDestination(info_dest, _flags) => {
                                dest_guid_prefix =
                                  if info_dest.guid_prefix != GUID::GUID_UNKNOWN.prefix {
                                    self.own_guid_prefix
                                  } else {
                                    info_dest.guid_prefix
                                  };
                              }
                              _ => {}
                            }
                          }

                          #[cfg(feature = "rtps_proxy")]
                          {
                            proxy_the_secure_submessages = match &decoded_msg {
                              DecodedSubmessage::Reader(msg, _handles) => {
                                !ENTITY_IDS_WITH_NO_DIRECT_RTPS_PROXYING
                                  .contains(&msg.sender_entity_id())
                              }
                              DecodedSubmessage::Writer(msg, _handles) => {
                                !ENTITY_IDS_WITH_NO_DIRECT_RTPS_PROXYING
                                  .contains(&msg.sender_entity_id())
                              }
                              DecodedSubmessage::Interpreter(_msg) => true,
                            };
                          }

                          unprotected_submessages.push(UnprotectedSubmessage::from(decoded_msg));
                        }
                      }

                      #[cfg(feature = "rtps_proxy")]
                      if proxy_the_secure_submessages {
                        proxied_submessages.append(&mut secure_submessages_for_proxying);
                      }
                    } else {
                      // The prefix submessage was not a Secure prefix. Should not happen.
                      error!("Found an invalid submessage in the secure receiver state");
                      secure_receiver_state = SecureReceiverState::None;
                    }
                  }
                }
              }
              SecuritySubmessage::SecureRTPSPrefix(..)
              | SecuritySubmessage::SecureRTPSPostfix(..) => {
                // SecureRTPSPrefix and SecureRTPSPostfix should have been removed when decoding
                // RTPS-level protection
                warn!("Unexpected SecureRTPSPrefix/SecureRTPSPostfix submessage. Discarding.");
              }
            } // match sec_submsg
          }
        }
      }
    }

    #[cfg(feature = "rtps_proxy")]
    self.proxy_rtps_message(Message {
      header: rtps_message.header,
      submessages: proxied_submessages,
    });

    unprotected_submessages
  }

  fn handle_unprotected_submessage(&mut self, submessage: UnprotectedSubmessage) {
    match submessage {
      UnprotectedSubmessage::Interpreter(submsg) => {
        self.handle_interpreter_submessage(submsg);
      }
      UnprotectedSubmessage::Writer(writer_submsg, allowed_endpoints) => {
        let receiver_entity_id = writer_submsg.receiver_entity_id();

        let potential_reader_entity_ids = match receiver_entity_id {
          EntityId::UNKNOWN => {
            // We have to find which local readers are matched to this writer
            let sending_writer_entity_id = writer_submsg.sender_entity_id();
            self
            .available_readers
            .values()
            .filter(|target_reader| {
              // Reader must contain the writer
              target_reader.contains_writer(sending_writer_entity_id)
                // But there are two exceptions:
                // 1. SPDP reader must read from unknown SPDP writers
                //  TODO: This logic here is uglyish. Can we just inject a
                //  presupposed writer (proxy) to the built-in reader as it is created?
                || (sending_writer_entity_id == EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER
                  && target_reader.entity_id() == EntityId::SPDP_BUILTIN_PARTICIPANT_READER)
                // 2. ParticipantStatelessReader does not contain any writers, since it is stateless
                || (sending_writer_entity_id == EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_WRITER
                  && target_reader.entity_id() == EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_READER)
            })
            .map(Reader::entity_id).collect()
          }
          other => vec![other],
        };

        match potential_reader_entity_ids.as_slice() {
          &[reader_entity_id] => {
            // Just one reader, avoid cloning the writer submessage
            self.handle_writer_submessage(reader_entity_id, writer_submsg, &allowed_endpoints);
          }
          _ => {
            // Could be multiple readers. Clone the submessage for each one
            for reader_entity_id in potential_reader_entity_ids {
              self.handle_writer_submessage(
                reader_entity_id,
                writer_submsg.clone(),
                &allowed_endpoints,
              );
            }
          }
        }
      }
      UnprotectedSubmessage::Reader(submsg, allowed_endpoints) => {
        self.handle_reader_submessage(submsg, &allowed_endpoints);
      }
    }
  }

  fn handle_writer_submessage(
    &mut self,
    target_reader_entity_id: EntityId,
    submessage: WriterSubmessage,
    // The use of conditional compilation here is clumsy. How to do it better?
    #[cfg(not(feature = "security"))] _allowed_target_readers: &AllowedEndpoints,
    #[cfg(feature = "security")] allowed_target_readers: &AllowedEndpoints,
  ) {
    if self.dest_guid_prefix != self.own_guid_prefix && self.dest_guid_prefix != GuidPrefix::UNKNOWN
    {
      debug!(
        "Message is not for this participant. Dropping. dest_guid_prefix={:?} participant \
         guid={:?}",
        self.dest_guid_prefix, self.own_guid_prefix
      );
      return;
    }

    // Check can we give the submessage to the reader, security-wise
    #[cfg(not(feature = "security"))]
    let allowed_for_this_reader = true;

    #[cfg(feature = "security")]
    let allowed_for_this_reader = {
      // Check that both RTPS protection and submessage protection are OK

      let rtps_protection_ok = if self.must_be_rtps_protection_special_case {
        // RTPS protection is enabled, but the WriterSubmessage is from a non-protected
        // RTPS message. The reader needs to be one of those built-in readers
        // that are allowed accept data from non-protected RTPS messages.
        [
          EntityId::SPDP_BUILTIN_PARTICIPANT_READER,
          EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_READER,
          EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_READER,
        ]
        .contains(&target_reader_entity_id)
      } else {
        true
      };

      let reader_guid =
        GUID::new_with_prefix_and_id(self.dest_guid_prefix, target_reader_entity_id);

      let submessage_protection_ok = match allowed_target_readers {
        AllowedEndpoints::WithNoSubmessageProtection => {
          match self.security_plugins.as_ref() {
            None => true, // Security not enabled, always allow
            Some(plugins_handle) => {
              // The reader's topic cannot be submessage-protected
              plugins_handle
                .get_plugins()
                .submessage_not_protected(&reader_guid)
            }
          }
        }
        AllowedEndpoints::WithCryptoHandles(handles) => {
          // Verify that one of the crypto handles is the reader's
          match self.security_plugins.as_ref() {
            None => false, // Should not be possible
            Some(plugins_handle) => plugins_handle
              .get_plugins()
              .confirm_local_endpoint_guid(handles, &reader_guid),
          }
        }
      };

      rtps_protection_ok && submessage_protection_ok
    };

    if !allowed_for_this_reader {
      warn!(
        "Cannot give a WriterSubmessage to reader with id {:?} because of security concerns",
        target_reader_entity_id
      );
      return;
    }

    let mr_state = self.clone_partial_message_receiver_state();
    let writer_entity_id = submessage.sender_entity_id();
    let source_guid_prefix = mr_state.source_guid_prefix;
    let source_guid = &GUID {
      prefix: source_guid_prefix,
      entity_id: writer_entity_id,
    };

    let security_plugins = self.security_plugins.clone();

    let target_reader = if let Some(target_reader) = self.reader_mut(target_reader_entity_id) {
      target_reader
    } else {
      return error!("No reader matching the CryptoHandle found");
    };

    match submessage {
      WriterSubmessage::Data(data, data_flags) => {
        Self::decode_and_handle_data(
          security_plugins.as_ref(),
          source_guid,
          data,
          data_flags,
          target_reader,
          &mr_state,
        );

        // Notify discovery that the remote PArticipant seems to be alive
        if writer_entity_id == EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER
          && target_reader_entity_id == EntityId::SPDP_BUILTIN_PARTICIPANT_READER
        {
          self
            .spdp_liveness_sender
            .try_send(source_guid_prefix)
            .unwrap_or_else(|e| {
              debug!(
                "spdp_liveness_sender.try_send(): {:?}. Is Discovery alive?",
                e
              );
            });
        }
      }

      WriterSubmessage::Heartbeat(heartbeat, flags) => {
        target_reader.handle_heartbeat_msg(
          &heartbeat,
          flags.contains(HEARTBEAT_Flags::Final),
          &mr_state,
        );
      }

      WriterSubmessage::Gap(gap, _flags) => {
        target_reader.handle_gap_msg(&gap, &mr_state);
      }

      WriterSubmessage::DataFrag(datafrag, flags) => {
        Self::decode_and_handle_datafrag(
          security_plugins.as_ref(),
          source_guid,
          datafrag.clone(),
          flags,
          target_reader,
          &mr_state,
        );
      }

      WriterSubmessage::HeartbeatFrag(heartbeatfrag, _flags) => {
        target_reader.handle_heartbeatfrag_msg(&heartbeatfrag, &mr_state);
      }
    }
  }

  // see security version of the same function below
  #[cfg(not(feature = "security"))]
  fn decode_and_handle_data(
    _security_plugins: Option<&SecurityPluginsHandle>,
    _source_guid: &GUID,
    data: Data,
    data_flags: BitFlags<DATA_Flags, u8>,
    reader: &mut Reader,
    mr_state: &MessageReceiverState,
  ) {
    reader.handle_data_msg(data, data_flags, mr_state);
  }

  #[cfg(feature = "security")]
  fn decode_and_handle_data(
    security_plugins: Option<&SecurityPluginsHandle>,
    source_guid: &GUID,
    data: Data,
    data_flags: BitFlags<DATA_Flags, u8>,
    reader: &mut Reader,
    mr_state: &MessageReceiverState,
  ) {
    let Data {
      inline_qos,
      serialized_payload,
      ..
    } = data.clone();

    serialized_payload
      // If there is an encoded_payload, decode it
      .map(
        |encoded_payload| match security_plugins.map(SecurityPluginsHandle::get_plugins) {
          Some(security_plugins) => security_plugins
            .decode_serialized_payload(
              encoded_payload,
              inline_qos.unwrap_or_default(),
              source_guid,
              &reader.guid(),
            )
            .map_err(|e| error!("{e:?}")),
          None => Ok(encoded_payload),
        },
      )
      .transpose()
      // If there were no errors, give to the reader
      .map(|decoded_payload| {
        reader.handle_data_msg(
          Data {
            serialized_payload: decoded_payload,
            ..data
          },
          data_flags,
          mr_state,
        );
      })
      // Errors have already been printed
      .ok();
  }

  #[cfg(not(feature = "security"))]
  // see security version below
  fn decode_and_handle_datafrag(
    _security_plugins: Option<&SecurityPluginsHandle>,
    _source_guid: &GUID,
    datafrag: DataFrag,
    datafrag_flags: BitFlags<DATAFRAG_Flags, u8>,
    reader: &mut Reader,
    mr_state: &MessageReceiverState,
  ) {
    let payload_buffer_length = datafrag.serialized_payload.len();
    if payload_buffer_length
      > (datafrag.fragments_in_submessage as usize) * (datafrag.fragment_size as usize)
    {
      error!(
        "{:?}",
        std::io::Error::new(
          std::io::ErrorKind::Other,
          format!(
            "Invalid DataFrag. serializedData length={} should be less than or equal to \
             (fragments_in_submessage={}) x (fragment_size={})",
            payload_buffer_length, datafrag.fragments_in_submessage, datafrag.fragment_size
          ),
        )
      );
      // and we're done
    } else {
      reader.handle_datafrag_msg(&datafrag, datafrag_flags, mr_state);
    }
    // Consume to keep the same method signature as in the security case
    drop(datafrag);
  }

  #[cfg(feature = "security")]
  fn decode_and_handle_datafrag(
    security_plugins: Option<&SecurityPluginsHandle>,
    source_guid: &GUID,
    datafrag: DataFrag,
    datafrag_flags: BitFlags<DATAFRAG_Flags, u8>,
    reader: &mut Reader,
    mr_state: &MessageReceiverState,
  ) {
    let DataFrag {
      inline_qos,
      serialized_payload: encoded_payload,
      ..
    } = datafrag.clone();

    match security_plugins.map(SecurityPluginsHandle::get_plugins) {
      Some(security_plugins) => {
        // Decode
        security_plugins
          .decode_serialized_payload(
            encoded_payload,
            inline_qos.unwrap_or_default(),
            source_guid,
            &reader.guid(),
          )
          .map_err(|e| error!("{e:?}"))
      }
      None => Ok(encoded_payload),
    }
    .ok()
    // Deserialize
    .and_then(|serialized_payload| {
      // The check that used to be in DataFrag deserialization
      if serialized_payload.len()
        > (datafrag.fragments_in_submessage as usize) * (datafrag.fragment_size as usize)
      {
        error!(
          "{:?}",
          std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
              "Invalid DataFrag. serializedData length={} should be less than or equal to \
               (fragments_in_submessage={}) x (fragment_size={})",
              serialized_payload.len(),
              datafrag.fragments_in_submessage,
              datafrag.fragment_size
            ),
          )
        );
        None
      } else {
        Some(serialized_payload)
      }
    })
    // If there were no errors, give DecodedDataFrag to the reader
    .map(|decoded_payload| {
      reader.handle_datafrag_msg(
        &DataFrag {
          serialized_payload: decoded_payload,
          ..datafrag
        },
        datafrag_flags,
        mr_state,
      );
    });
  }

  fn handle_reader_submessage(
    &self,
    submessage: ReaderSubmessage,
    // The use of conditional compilation here is clumsy. How to do it better?
    #[cfg(not(feature = "security"))] _allowed_target_writers: &AllowedEndpoints,
    #[cfg(feature = "security")] allowed_target_writers: &AllowedEndpoints,
  ) {
    if self.dest_guid_prefix != self.own_guid_prefix && self.dest_guid_prefix != GuidPrefix::UNKNOWN
    {
      debug!(
        "Message is not for this participant. Dropping. dest_guid_prefix={:?} participant \
         guid={:?}",
        self.dest_guid_prefix, self.own_guid_prefix
      );
      return;
    }

    let writer_guid =
      GUID::new_with_prefix_and_id(self.dest_guid_prefix, submessage.receiver_entity_id());

    // Check can we are allowed to process this message, security-wise
    #[cfg(not(feature = "security"))]
    let allowed_to_process = true;

    #[cfg(feature = "security")]
    let allowed_to_process = {
      // Check that both RTPS protection and submessage protection are OK

      let rtps_protection_ok = if self.must_be_rtps_protection_special_case {
        // RTPS protection is enabled, but the ReaderSubmessage is from a non-protected
        // RTPS message. The writer needs to be one of those built-in writers
        // that are allowed accept data from non-protected RTPS messages.
        [
          EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER,
          EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_WRITER,
          EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_WRITER,
        ]
        .contains(&writer_guid.entity_id)
      } else {
        true
      };

      let submessage_protection_ok = match allowed_target_writers {
        AllowedEndpoints::WithNoSubmessageProtection => {
          match self.security_plugins.as_ref() {
            None => true, // Security not enabled, always allow
            Some(plugins_handle) => {
              // The writer's topic cannot be submessage-protected
              plugins_handle
                .get_plugins()
                .submessage_not_protected(&writer_guid)
            }
          }
        }
        AllowedEndpoints::WithCryptoHandles(handles) => {
          // Verify that one of the crypto handles is the writer's
          match self.security_plugins.as_ref() {
            None => false, // Should not be possible
            Some(plugins_handle) => plugins_handle
              .get_plugins()
              .confirm_local_endpoint_guid(handles, &writer_guid),
          }
        }
      };

      rtps_protection_ok && submessage_protection_ok
    };

    if !allowed_to_process {
      warn!(
        "Cannot process a ReaderSubmessage sent to {:?} because of security concerns",
        writer_guid
      );
      return;
    }

    match submessage {
      ReaderSubmessage::AckNack(acknack, _) => {
        // Note: This must not block, because the receiving end is the same thread,
        // i.e. blocking here is an instant deadlock.
        match self
          .acknack_sender
          .try_send((self.source_guid_prefix, AckSubmessage::AckNack(acknack)))
        {
          Ok(_) => (),
          Err(TrySendError::Full(_)) => {
            info!("AckNack pipe full. Looks like I am very busy. Discarding submessage.");
          }
          Err(e) => warn!("AckNack pipe fail: {:?}", e),
        }
      }

      ReaderSubmessage::NackFrag(_, _) => {
        // TODO: Implement NackFrag handling
      }
    }
  }

  fn handle_interpreter_submessage(&mut self, interpreter_submessage: InterpreterSubmessage)
  // no return value, just change state of self.
  {
    match interpreter_submessage {
      InterpreterSubmessage::InfoTimestamp(ts_struct, _flags) => {
        // flags value was used already when parsing timestamp into an Option
        self.source_timestamp = ts_struct.timestamp;
      }
      InterpreterSubmessage::InfoSource(info_src, _flags) => {
        self.source_guid_prefix = info_src.guid_prefix;
        self.source_version = info_src.protocol_version;
        self.source_vendor_id = info_src.vendor_id;

        // TODO: Why are the following set on InfoSource?
        self.unicast_reply_locator_list.clear(); // Or invalid?
        self.multicast_reply_locator_list.clear(); // Or invalid?
        self.source_timestamp = None; // TODO: Why does InfoSource set timestamp
                                      // to None?
      }
      InterpreterSubmessage::InfoReply(info_reply, flags) => {
        self.unicast_reply_locator_list = info_reply.unicast_locator_list;
        self.multicast_reply_locator_list = match (
          flags.contains(INFOREPLY_Flags::Multicast),
          info_reply.multicast_locator_list,
        ) {
          (true, Some(ll)) => ll, // expected case
          (true, None) => {
            warn!(
              "InfoReply submessage flag indicates multicast_reply_locator_list, but none found."
            );
            vec![]
          }
          (false, None) => vec![], // This one is normal again
          (false, Some(_)) => {
            warn!("InfoReply submessage has unexpected multicast_reply_locator_list, ignoring.");
            vec![]
          }
        };
      }
      InterpreterSubmessage::InfoDestination(info_dest, _flags) => {
        if info_dest.guid_prefix == GUID::GUID_UNKNOWN.prefix {
          self.dest_guid_prefix = self.own_guid_prefix;
        } else {
          self.dest_guid_prefix = info_dest.guid_prefix;
        }
      }
    }
  }

  pub fn notify_data_to_readers(&mut self, readers: Vec<EntityId>) {
    for eid in readers {
      self
        .available_readers
        .get_mut(&eid)
        .map(Reader::notify_cache_change);
    }
  }

  // sends 0 seqnum acknacks for those writer that haven't had any action
  pub fn send_preemptive_acknacks(&mut self) {
    for reader in self.available_readers.values_mut() {
      reader.send_preemptive_acknacks();
    }
  }

  #[cfg(feature = "rtps_proxy")]
  fn proxy_rtps_message(&self, rtps_message: Message) {
    // Filter out submessages of those topics that should not be directly
    // RTPS-proxied, and send the resulting RTPS message to the proxy

    let mut contains_other_than_interpreter_messages = false;

    let filtered_submessages: Vec<Submessage> = rtps_message
      .submessages
      .into_iter()
      .filter(|submsg| match &submsg.body {
        SubmessageBody::Reader(msg) => {
          let should_be_proxied =
            !ENTITY_IDS_WITH_NO_DIRECT_RTPS_PROXYING.contains(&msg.sender_entity_id());
          if should_be_proxied {
            contains_other_than_interpreter_messages = true;
          }
          should_be_proxied
        }
        SubmessageBody::Writer(msg) => {
          let should_be_proxied =
            !ENTITY_IDS_WITH_NO_DIRECT_RTPS_PROXYING.contains(&msg.sender_entity_id());
          if should_be_proxied {
            contains_other_than_interpreter_messages = true;
          }
          should_be_proxied
        }
        SubmessageBody::Interpreter(_msg) => true,
        #[cfg(feature = "security")]
        SubmessageBody::Security(_msg) => {
          contains_other_than_interpreter_messages = true;
          true
        }
      })
      .collect();

    if contains_other_than_interpreter_messages {
      let proxied_message = Message {
        header: rtps_message.header,
        submessages: filtered_submessages,
      };

      if let Err(e) = self.proxy_rtps_sender.try_send(rtps_proxy::RTPSMessage {
        msg: proxied_message,
      }) {
        error!("RTPS proxy: failed to send RTPS message to proxy: {e}");
      }
    }
  }

  // use for test and debugging only
  #[cfg(test)]
  fn get_reader_and_history_cache_change(
    &self,
    reader_id: EntityId,
    sequence_number: SequenceNumber,
  ) -> Option<DDSData> {
    Some(
      self
        .available_readers
        .get(&reader_id)
        .unwrap()
        .history_cache_change_data(sequence_number)
        .unwrap(),
    )
  }

  #[cfg(test)]
  fn get_reader_history_cache_start_and_end_seq_num(
    &self,
    reader_id: EntityId,
  ) -> Vec<SequenceNumber> {
    self
      .available_readers
      .get(&reader_id)
      .unwrap()
      .history_cache_sequence_start_and_end_numbers()
  }
} // impl messageReceiver

// ------------------------------------------------------------------------------------------------------
// ------------------------------------------------------------------------------------------------------
// ------------------------------------------------------------------------------------------------------
// ------------------------------------------------------------------------------------------------------
// ------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::{
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
  };

  use speedy::{Readable, Writable};
  use log::info;
  use serde::{Deserialize, Serialize};
  use mio_extras::channel as mio_channel;

  use crate::{
    dds::{
      qos::QosPolicies,
      statusevents::{sync_status_channel, DataReaderStatus},
      typedesc::TypeDesc,
      with_key::simpledatareader::ReaderCommand,
    },
    messages::header::Header,
    mio_source,
    network::udp_sender::UDPSender,
    rtps::reader::ReaderIngredients,
    serialization::cdr_deserializer::deserialize_from_little_endian,
    structure::{dds_cache::DDSCache, guid::EntityKind},
  };
  use super::*;
  #[cfg(feature = "rtps_proxy")]
  use crate::rtps_proxy::sync_proxy_data_channel;

  #[test]

  fn test_shapes_demo_message_deserialization() {
    // The following message bytes contain serialized INFO_DST, INFO_TS, DATA &
    // HEARTBEAT submessages. The DATA submessage contains a ShapeType value.
    // The bytes have been captured from WireShark.
    let udp_bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08, 0x00, 0x1a, 0x15, 0xf3, 0x5e, 0x00,
      0xcc, 0xfb, 0x13, 0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07,
      0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00,
      0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00,
      0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x5b, 0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00,
    ]);

    // The message bytes contain the following guid prefix as the message target.
    let target_gui_prefix = GuidPrefix::new(&[
      0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31, 0xa2, 0x28, 0x20, 0x02, 0x08,
    ]);

    // The message bytes contain the following guid as the message source
    let remote_writer_guid = GUID::new(
      GuidPrefix::new(&[
        0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
      ]),
      EntityId::create_custom_entity_id([0, 0, 1], EntityKind::WRITER_WITH_KEY_USER_DEFINED),
    );

    #[cfg(feature = "rtps_proxy")]
    let (proxy_data_sender, _proxy_data_receiver) = sync_proxy_data_channel(16).unwrap();

    // Create a message receiver
    let (acknack_sender, _acknack_receiver) =
      mio_channel::sync_channel::<(GuidPrefix, AckSubmessage)>(10);
    let (spdp_liveness_sender, _spdp_liveness_receiver) = mio_channel::sync_channel(8);
    let mut message_receiver = MessageReceiver::new(
      target_gui_prefix,
      acknack_sender,
      spdp_liveness_sender,
      None,
      #[cfg(feature = "rtps_proxy")]
      proxy_data_sender,
    );

    // Create a reader to process the message
    let entity =
      EntityId::create_custom_entity_id([0, 0, 0], EntityKind::READER_WITH_KEY_USER_DEFINED);
    let reader_guid = GUID::new_with_prefix_and_id(target_gui_prefix, entity);

    let (notification_sender, _notification_receiver) = mio_channel::sync_channel::<()>(100);
    let (_notification_event_source, notification_event_sender) =
      mio_source::make_poll_channel().unwrap();
    let data_reader_waker = Arc::new(Mutex::new(None));

    let (status_sender, _status_receiver) = sync_status_channel::<DataReaderStatus>(4).unwrap();
    let (participant_status_sender, _participant_status_receiver) =
      sync_status_channel(16).unwrap();

    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let qos_policy = QosPolicies::qos_none();

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));

    let topic_cache_handle = dds_cache.write().unwrap().add_new_topic(
      "test".to_string(),
      TypeDesc::new("test".to_string()),
      &qos_policy,
    );
    let reader_ing = ReaderIngredients {
      guid: reader_guid,
      notification_sender,
      status_sender,
      topic_name: "test".to_string(),
      topic_cache_handle: topic_cache_handle.clone(),
      like_stateless: false,
      qos_policy,
      data_reader_command_receiver: reader_command_receiver,
      data_reader_waker: data_reader_waker.clone(),
      poll_event_sender: notification_event_sender,
      security_plugins: None,
    };

    let mut new_reader = Reader::new(
      reader_ing,
      Rc::new(UDPSender::new_with_random_port().unwrap()),
      mio_extras::timer::Builder::default().build(),
      participant_status_sender,
    );

    // Add info of the writer to the reader
    new_reader.matched_writer_add(
      remote_writer_guid,
      EntityId::UNKNOWN,
      vec![],
      vec![],
      &QosPolicies::qos_none(),
    );

    // Add reader to message reader and process the bytes message
    message_receiver.add_reader(new_reader);

    message_receiver.handle_received_packet(&udp_bits1);

    // Verify the message reader has recorded the right amount of submessages
    assert_eq!(message_receiver.submessage_count, 4);

    // This is not correct way to read history cache values but it serves as a test
    let sequence_numbers =
      message_receiver.get_reader_history_cache_start_and_end_seq_num(reader_guid.entity_id);
    info!(
      "history change sequence number range: {:?}",
      sequence_numbers
    );

    // Get the DDSData (serialized) from the topic cache / history cache
    let a = message_receiver
      .get_reader_and_history_cache_change(
        reader_guid.entity_id,
        *sequence_numbers.first().unwrap(),
      )
      .expect("No data in topic cache");
    info!("reader history cache DATA: {:?}", a.data());

    // Deserialize the ShapesType value from the data
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct ShapeType {
      color: String,
      x: i32,
      y: i32,
      size: i32,
    }
    let deserialized_shape_type: ShapeType = deserialize_from_little_endian(&a.data()).unwrap();
    info!("deserialized shapeType: {:?}", deserialized_shape_type);

    // Verify the color in the deserialized value is correct
    assert_eq!(deserialized_shape_type.color, "RED");
  }

  #[test]
  fn mr_test_submsg_count() {
    // Udp packet with INFO_DST, INFO_TS, DATA, HEARTBEAT
    let udp_bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08, 0x00, 0x18, 0x15, 0xf3, 0x5e, 0x00,
      0x5c, 0xf0, 0x34, 0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07,
      0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x21, 0x00, 0x00, 0x00, 0x89, 0x00,
      0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00,
      0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x43, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
    ]);
    // Udp packet with INFO_DST, ACKNACK
    let udp_bits2 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x06, 0x03, 0x18, 0x00, 0x00, 0x00, 0x04, 0xc7, 0x00,
      0x00, 0x04, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x03, 0x00, 0x00, 0x00,
    ]);

    let guid_new = GUID::default();
    let (acknack_sender, _acknack_receiver) =
      mio_channel::sync_channel::<(GuidPrefix, AckSubmessage)>(10);
    let (spdp_liveness_sender, _spdp_liveness_receiver) = mio_channel::sync_channel(8);

    #[cfg(feature = "rtps_proxy")]
    let (proxy_data_sender, _proxy_data_receiver) = sync_proxy_data_channel(16).unwrap();

    let mut message_receiver = MessageReceiver::new(
      guid_new.prefix,
      acknack_sender,
      spdp_liveness_sender,
      None,
      #[cfg(feature = "rtps_proxy")]
      proxy_data_sender,
    );

    message_receiver.handle_received_packet(&udp_bits1);
    assert_eq!(message_receiver.submessage_count, 4);

    message_receiver.handle_received_packet(&udp_bits2);
    assert_eq!(message_receiver.submessage_count, 2);
  }

  #[test]
  fn mr_test_header() {
    let guid_new = GUID::default();
    let header = Header::new(guid_new.prefix);

    let bytes = header.write_to_vec().unwrap();
    let new_header = Header::read_from_buffer(&bytes).unwrap();
    assert_eq!(header, new_header);
  }
}
