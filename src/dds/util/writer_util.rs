use crate::{
  dds::ddsdata::DDSData, dds::qos::HasQoSPolicy, dds::qos::policy::History, dds::writer::Writer,
  structure::cache_change::CacheChange, structure::entity::Entity,
  structure::sequence_number::SequenceNumber,
};

pub struct WriterUtil {}

impl WriterUtil {
  pub fn increment_writer_sequence_number(writer: &mut Writer) {
    // first increasing last SequenceNumber
    writer.last_change_sequence_number =
      writer.last_change_sequence_number + SequenceNumber::from(1);

    // setting first change sequence number according to our qos (not offering more than our QOS says)
    match writer.get_qos().history {
      Some(hs) => {
        match hs {
          History::KeepAll => {
            // If first write then set first change sequence number to 1
            if writer.first_change_sequence_number == SequenceNumber::from(0) {
              writer.first_change_sequence_number = SequenceNumber::from(1);
            }
          }
          History::KeepLast { depth } => {
            writer.first_change_sequence_number =
              if i64::from(writer.last_change_sequence_number) - ((depth as i64) - 1) > 0 {
                writer.last_change_sequence_number - SequenceNumber::from((depth as i64) - 1)
              } else {
                SequenceNumber::from(1)
              }
          }
        }
      }
      // Depth 1
      None => {
        writer.first_change_sequence_number = writer.last_change_sequence_number;
      }
    };
  }

  pub fn create_cache_change_from_dds_data(writer: &Writer, data: DDSData) -> CacheChange {
    let change_kind = data.change_kind;
    let datavalue = Some(data);
    let new_cache_change = CacheChange::new(
      change_kind,
      writer.get_guid(),
      writer.last_change_sequence_number,
      datavalue,
    );
    new_cache_change
  }
}
