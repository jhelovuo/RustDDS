use crate::messages::fragment_number::FragmentNumber;
use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;
use speedy::{Readable, Writable};

/// The DataFrag Submessage extends the Data Submessage by enabling the
/// serializedData to be fragmented and sent as multiple DataFrag Submessages.
/// The fragments contained in the DataFrag Submessages are then re-assembled by
/// the RTPS Reader.
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct DataFrag {
  /// Identifies the RTPS Reader entity that is being informed of the change
  /// to the data-object.
  pub reader_id: EntityId,

  /// Identifies the RTPS Writer entity that made the change to the
  /// data-object.
  pub writer_id: EntityId,

  /// Uniquely identifies the change and the relative order for all changes
  /// made by the RTPS Writer identified by the writerGuid.
  /// Each change gets a consecutive sequence number.
  /// Each RTPS Writer maintains is own sequence number.
  pub writer_sn: SequenceNumber,

  /// Indicates the starting fragment for the series of fragments in
  /// serialized_data. Fragment numbering starts with number 1.
  pub fragment_starting_num: FragmentNumber,

  /// The number of consecutive fragments contained in this Submessage,
  /// starting at fragment_starting_num.
  pub fragments_in_submessage: u16,

  /// The total size in bytes of the original data before fragmentation.
  pub data_size: u32,

  /// The size of an individual fragment in bytes. The maximum fragment size
  /// equals 64K.
  pub fragment_size: u16,

  /// Contains QoS that may affect the interpretation of the message.
  /// Present only if the InlineQosFlag is set in the header.
  pub inline_qos: ParameterList,

  /// Encapsulation of a consecutive series of fragments, starting at
  /// fragment_starting_num for a total of fragments_in_submessage.
  /// Represents part of the new value of the data-object
  /// after the change. Present only if either the DataFlag or the KeyFlag are
  /// set in the header. Present only if DataFlag is set in the header.
  pub serialized_payload: SerializedPayload,
}
