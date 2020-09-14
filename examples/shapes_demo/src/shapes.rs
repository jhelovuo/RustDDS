use atosdds::dds::traits::key::Keyed;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Square {
  color: String,
  x: i32,
  y: i32,
  shapesize: i32,
}

impl Square {
  pub fn new(color: String, x: i32, y: i32, shapesize: i32) -> Square {
    Square {
      color,
      x,
      y,
      shapesize,
    }
  }

  pub fn xadd(&mut self, d: i32) {
    self.x += d;
  }

  pub fn yadd(&mut self, d: i32) {
    self.y += d;
  }
}

impl Keyed for Square {
  type K = String;

  fn get_key(&self) -> Self::K {
    self.color.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atosdds::{
    submessages::RepresentationIdentifier,
    serialization::cdrDeserializer::CDR_deserializer_adapter,
    dds::traits::serde_adapters::DeserializerAdapter,
    serialization::cdrSerializer::to_little_endian_binary,
  };

  #[test]
  fn foobar() {
    let data: [u8; 20] = [
      0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x10, 0x00, 0x00, 0x00, 0xa4, 0x00, 0x00,
      0x00, 0x1e, 0x00, 0x00, 0x00,
    ];

    let sq =
      CDR_deserializer_adapter::<Square>::from_bytes(&data, RepresentationIdentifier::CDR_LE)
        .unwrap();

    let sq2 = Square {
      color: String::from("RED"),
      x: 16,
      y: 164,
      shapesize: 30,
    };

    assert_eq!(sq, sq2);

    let data2 = to_little_endian_binary::<Square>(&sq2).unwrap();

    assert_eq!(data.to_vec(), data2);
  }
}
