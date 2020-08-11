use serde::{Serialize, Deserialize};

use crate::dds::traits::key::{Key,Keyed};

#[derive(Debug, Serialize, Deserialize)]
pub struct SPDPDiscoveredParticipantData {}
/*
impl DataSampleTrait for SPDPDiscoveredParticipantData {
  fn box_clone(&self) -> Box<dyn DataSampleTrait> {
    todo!()
  }
}
*/
impl Keyed for SPDPDiscoveredParticipantData {
  type K = u64; // placeholder
  fn get_key(&self) -> Self::K {
    todo!()
  }
}

impl Key for u64 {

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
