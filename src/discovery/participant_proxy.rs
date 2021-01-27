/*
use serde::{Serialize, Deserialize};

use crate::structure::{guid::*, locator::LocatorList};

use crate::messages::{protocol_version::ProtocolVersion, vendor_id::VendorId};

/// Defines ParticipantProxyAttributes

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantProxyAttributes {
  /// Identifies the DDS domainId of the associated DDS DomainParticipant
  pub domain_id: EntityId,

  /// Identifies the DDSTag of the associated DDS DomainParticipant
  pub domain_tag: String,

  /// Identifies the RTPS protocol version used by the Participant
  pub protocol_version: ProtocolVersion,

  /// The common GuidPrefix_t of the Participant and all the Endpoints contained
  /// within the Participant.
  pub guid_prefix: GuidPrefix,

  /// Identifies the vendor of the DDS middleware that contains the Participant.
  pub vendor_id: VendorId,

  /// Describes whether the Readers within the Participant
  /// expect that the QoS values that apply to each data
  /// modification are encapsulated included with each Data.
  pub expects_inline_qos: bool,

  /// List of unicast locators (transport, address, port
  /// combinations) that can be used to send messages to
  /// the built-in Endpoints contained in the Participant.
  pub metatraffic_unicast_locator_list: LocatorList,

  /// List of multicast locators (transport, address, port
  /// combinations) that can be used to send messages to
  /// the built-in Endpoints contained in the Participant.
  pub metatraffic_multicast_locator_list: LocatorList,

  /// Default list of unicast locators (transport, address, port
  /// combinations) that can be used to send messages to
  /// the user-defined Endpoints contained in the
  /// Participant.
  /// These are the unicast locators that will be used in case
  /// the Endpoint does not specify its own set of Locators, so
  /// at least one Locator must be present
  pub default_unicast_locator_list: LocatorList,

  /// Default list of multicast locators (transport, address,
  /// port combinations) that can be used to send messages to
  /// the user-defined Endpoints contained in the Participant.
  /// These are the multicast locators that will be used in case
  /// the Endpoint does not specify its own set of Locators.
  pub default_multicast_locator_list: LocatorList,

  /// All Participants must support the SEDP. This attribute
  /// identifies the kinds of built-in SEDP Endpoints that are
  /// available in the Participant. This allows a Participant to
  /// indicate that it only contains a subset of the possible
  /// built- in Endpoints. See also 8.5.4.3.
  /// Possible members in the BuiltinEndpointSet_t are:
  /// PUBLICATIONS_DETECTOR,
  /// PUBLICATIONS_ANNOUNCER,
  /// SUBSCRIPTIONS_DETECTOR,
  /// SUBSCRIPTIONS_ANNOUNCER,
  /// TOPICS_DETECTOR, TOPICS_ANNOUNCER
  /// PARTICIPANT_MESSAGE_READER
  /// PARTICIPANT_MESSAGE_WRITER
  /// Vendor specific extensions may
  //available_builtin_endpoints: BuiltinEndpointsSet,

  /// Used to implement MANUAL_BY_PARTICIPANT
  /// liveliness QoS.
  /// When liveliness is asserted, the manualLivelinessCount
  /// is incremented and a new
  /// SPDPdiscoveredParticipantDatais sent.
  pub manual_liveliness_count: i32,
}
*/
// pub trait ParticipantProxy {
//   fn as_participant_proxy(&self) -> &ParticipantProxyAttributes;
// }
