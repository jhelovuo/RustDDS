use atosdds::dds::traits::key::Keyed;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
