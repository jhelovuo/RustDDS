use serde::{Serialize, Deserialize};

use crate::dds::traits::{key::Keyed, datasample_trait::DataSampleTrait};

#[derive(Debug, Serialize, Deserialize)]
pub struct SPDPDiscoveredParticipantData {}

impl DataSampleTrait for SPDPDiscoveredParticipantData {
  fn box_clone(&self) -> Box<dyn DataSampleTrait> {
    todo!()
  }
}

impl Keyed for SPDPDiscoveredParticipantData {
  fn get_key(&self) -> Box<dyn crate::dds::traits::key::Key> {
    todo!()
  }
}

// #[cfg(test)]
// mod tests {
//   use super::*;
//   use std::fs;

//   #[test]
//   fn pdata_deserialize() {
//     let data = fs::read("RTPS_Discovery_data(p).bin")?;

//   }
// }
