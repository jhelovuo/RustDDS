use std::{
  collections::BTreeSet,
  net::{Ipv4Addr, SocketAddr},
};

#[allow(unused_imports)]
use log::{debug, error, trace, warn};

use crate::{
  dds::qos::QosPolicies,
  discovery::data_types::topic_data::DiscoveredReaderData,
  messages::submessages::submessage::AckSubmessage,
  network::{
    constant::{user_traffic_multicast_port, user_traffic_unicast_port},
    util::{get_local_multicast_locators, get_local_unicast_locators},
  },
  structure::{
    guid::{EntityId, EntityKind, GUID},
    locator::Locator,
    sequence_number::SequenceNumber,
  },
};
use super::reader::ReaderIngredients;

#[derive(Debug, PartialEq, Clone)]
/// ReaderProxy class represents the information an RTPS StatefulWriter
/// maintains on each matched RTPS Reader
pub(crate) struct RtpsReaderProxy {
  ///Identifies the remote matched RTPS Reader that is represented by the
  /// ReaderProxy
  pub remote_reader_guid: GUID,
  /// Identifies the group to which the matched Reader belongs
  pub remote_group_entity_id: EntityId,
  /// List of unicast locators (transport, address, port combinations) that can
  /// be used to send messages to the matched RTPS Reader. The list may be empty
  pub unicast_locator_list: Vec<Locator>,
  /// List of multicast locators (transport, address, port combinations) that
  /// can be used to send messages to the matched RTPS Reader. The list may be
  /// empty
  pub multicast_locator_list: Vec<Locator>,

  /// Specifies whether the remote matched RTPS Reader expects in-line QoS to be
  /// sent along with any data.
  pub expects_in_line_qos: bool,
  /// Specifies whether the remote Reader is responsive to the Writer
  pub is_active: bool,

  // Reader has positively acked all SequenceNumbers _before_ this.
  // This is directly the same as readerSNState.base in ACKNACK submessage.
  pub all_acked_before: SequenceNumber,

  // List of SequenceNumbers to be sent to Reader. Both unsent and requested by ACKNACK.
  pub unsent_changes: BTreeSet<SequenceNumber>,

  // true = send repair data messages due to NACKs, buffer messages by DataWriter
  // false = send data messages directly from DataWriter
  pub repair_mode: bool,
  pub qos: QosPolicies,
}

impl RtpsReaderProxy {
  pub fn new(remote_reader_guid: GUID, qos: QosPolicies) -> RtpsReaderProxy {
    RtpsReaderProxy {
      remote_reader_guid,
      remote_group_entity_id: EntityId::UNKNOWN,
      unicast_locator_list: Vec::default(),
      multicast_locator_list: Vec::default(),
      expects_in_line_qos: false,
      is_active: true,
      all_acked_before: SequenceNumber::zero(),
      unsent_changes: BTreeSet::new(),
      repair_mode: false,
      qos,
    }
  }

  pub fn qos(&self) -> &QosPolicies {
    &self.qos
  }

  pub fn from_reader(
    reader: &ReaderIngredients,
    domain_id: u16,
    participant_id: u16,
  ) -> RtpsReaderProxy {
    let unicast_locator_list =
      get_local_unicast_locators(user_traffic_unicast_port(domain_id, participant_id));

    let multicast_locator_list =
      get_local_multicast_locators(user_traffic_multicast_port(domain_id));

    RtpsReaderProxy {
      remote_reader_guid: reader.guid,
      remote_group_entity_id: EntityId::UNKNOWN, //TODO
      unicast_locator_list,
      multicast_locator_list,
      expects_in_line_qos: false,
      is_active: true,
      all_acked_before: SequenceNumber::zero(),
      unsent_changes: BTreeSet::new(),
      repair_mode: false,
      qos: reader.qos_policy.clone(),
    }
  }

  fn discovered_or_default(drd: &[Locator], default: &[Locator]) -> Vec<Locator> {
    if drd.is_empty() {
      default.to_vec()
    } else {
      drd.to_vec()
    }
  }

  pub fn from_discovered_reader_data(
    discovered_reader_data: &DiscoveredReaderData,
    default_unicast_locators: Vec<Locator>,
    default_multicast_locators: Vec<Locator>,
  ) -> RtpsReaderProxy {
    let unicast_locator_list = Self::discovered_or_default(
      &discovered_reader_data.reader_proxy.unicast_locator_list,
      &default_unicast_locators,
    );
    let multicast_locator_list = Self::discovered_or_default(
      &discovered_reader_data.reader_proxy.multicast_locator_list,
      &default_multicast_locators,
    );

    RtpsReaderProxy {
      remote_reader_guid: discovered_reader_data.reader_proxy.remote_reader_guid,
      remote_group_entity_id: EntityId::UNKNOWN, //TODO
      unicast_locator_list,
      multicast_locator_list,
      expects_in_line_qos: discovered_reader_data.reader_proxy.expects_inline_qos,
      is_active: true,
      all_acked_before: SequenceNumber::zero(),
      unsent_changes: BTreeSet::new(),
      repair_mode: false,
      qos: discovered_reader_data
        .subscription_topic_data
        .generate_qos(),
    }
  }

  pub fn update(&mut self, updated: &RtpsReaderProxy) {
    if self.remote_reader_guid == updated.remote_reader_guid {
      self.unicast_locator_list = updated.unicast_locator_list.clone();
      self.multicast_locator_list = updated.multicast_locator_list.clone();
      self.expects_in_line_qos = updated.expects_in_line_qos;
    }
  }

  pub fn new_for_unit_testing(port_number: u16) -> RtpsReaderProxy {
    let mut unicast_locators = Vec::default();
    let locator = Locator::from(SocketAddr::new(
      std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
      port_number,
    ));
    unicast_locators.push(locator);
    RtpsReaderProxy {
      remote_reader_guid: GUID::dummy_test_guid(EntityKind::READER_WITH_KEY_USER_DEFINED),
      remote_group_entity_id: EntityId::UNKNOWN,
      unicast_locator_list: unicast_locators,
      multicast_locator_list: Vec::default(),
      expects_in_line_qos: false,

      is_active: true,
      all_acked_before: SequenceNumber::zero(),
      unsent_changes: BTreeSet::new(),
      repair_mode: false,
      qos: QosPolicies::qos_none(),
    }
  }

  pub fn can_send(&self) -> bool {
    !self.unsent_changes.is_empty()
  }

  pub fn handle_ack_nack(
    &mut self,
    ack_submessage: &AckSubmessage,
    last_available: SequenceNumber,
  ) {
    match ack_submessage {
      AckSubmessage::AckNack(acknack) => {
        self.all_acked_before = acknack.reader_sn_state.base();
        // clean up unsent_changes:
        // The handy split_off function "Returns everything after the given key,
        // including the key."
        self.unsent_changes = self.unsent_changes.split_off(&self.all_acked_before);

        // Insert the requested changes.
        for nack_sn in acknack.reader_sn_state.iter() {
          self.unsent_changes.insert(nack_sn);
        }
        // sanity check
        if let Some(&high) = self.unsent_changes.iter().next_back() {
          if high > last_available {
            warn!(
              "ReaderProxy {:?} asks for {:?} but I have only up to {:?}. ACKNACK = {:?}",
              self.remote_reader_guid, self.unsent_changes, last_available, acknack
            );
          }
        }
      }

      AckSubmessage::NackFrag(_nack_frag) => {
        // TODO
        error!("NACKFRAG not implemented")
      }
    }
  }

  /// this should be called everytime a new CacheChange is set to RTPS writer
  /// HistoryCache
  pub fn notify_new_cache_change(&mut self, sequence_number: SequenceNumber) {
    if sequence_number == SequenceNumber::from(0) {
      error!(
        "new cache change with {:?}! bad! my GUID = {:?}",
        sequence_number, self.remote_reader_guid
      );
    }
    self.unsent_changes.insert(sequence_number);
  }

  // pub fn remove_unsent_cache_change(&mut self, sequence_number: SequenceNumber)
  // {   self.unsent_changes.remove(&sequence_number);
  // }

  // pub fn sequence_is_acked(&self, sequence_number: SequenceNumber) -> bool {
  //   sequence_number < self.all_acked_before
  // }

  pub fn acked_up_to_before(&self) -> SequenceNumber {
    self.all_acked_before
  }

  pub fn content_is_equal(&self, other: &RtpsReaderProxy) -> bool {
    self.remote_reader_guid == other.remote_reader_guid
      && self.remote_group_entity_id == other.remote_group_entity_id
      && self.unicast_locator_list == other.unicast_locator_list
      && self.multicast_locator_list == other.multicast_locator_list
      && self.expects_in_line_qos == other.expects_in_line_qos
      && self.is_active == other.is_active
  }
}

// pub enum ChangeForReaderStatusKind {
//   UNSENT,
//   NACKNOWLEDGED,
//   REQUESTED,
//   ACKNOWLEDGED,
//   UNDERWAY,
// }

// ///The RTPS ChangeForReader is an association class that maintains
// information of a CacheChange in the RTPS ///Writer HistoryCache as it
// pertains to the RTPS Reader represented by the ReaderProxy pub struct
// RTPSChangeForReader {   ///Indicates the status of a CacheChange relative to
// the RTPS Reader represented by the ReaderProxy.   pub kind:
// ChangeForReaderStatusKind,   ///Indicates whether the change is relevant to
// the RTPS Reader represented by the ReaderProxy.   pub is_relevant: bool,
// }

// impl RTPSChangeForReader {
//   pub fn new() -> RTPSChangeForReader {
//     RTPSChangeForReader {
//       kind: ChangeForReaderStatusKind::UNSENT,
//       is_relevant: true,
//     }
//   }
// }
