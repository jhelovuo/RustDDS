use serde::{
  de::{Visitor, Error},
};
use super::{
  builtin_data_deserializer::BuiltinDataDeserializer, cdrDeserializer::CDR_deserializer_adapter,
};
use crate::discovery::data_types::spdp_participant_data::SPDPDiscoveredParticipantData;

use crate::{messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier};

use crate::dds::traits::serde_adapters::DeserializerAdapter;

use std::convert::TryFrom;

impl<'de> Visitor<'de> for BuiltinDataDeserializer {
  type Value = BuiltinDataDeserializer;
  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "ParameterId")
  }

  fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
  where
    E: Error,
  {
    let rep: RepresentationIdentifier = match CDR_deserializer_adapter::<u16>::from_bytes(
      &v[..2],
      RepresentationIdentifier::CDR_LE,
    ) {
      Ok(v) => match RepresentationIdentifier::try_from(v) {
        Ok(v) => v,
        _ => return Err(E::missing_field("representation identifier")),
      },
      Err(_) => return Err(E::missing_field("representation identifier")),
    };

    match rep {
      RepresentationIdentifier::PL_CDR_LE => Ok(self.parse_data_little_endian(&v[2..])),
      RepresentationIdentifier::PL_CDR_BE => Ok(self.parse_data_big_endian(&v[2..])),
      _ => Err(E::missing_field("representation identifier")),
    }
  }
}

impl<'de> Visitor<'de> for SPDPDiscoveredParticipantData {
  type Value = SPDPDiscoveredParticipantData;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "ParameterId")
  }
}
