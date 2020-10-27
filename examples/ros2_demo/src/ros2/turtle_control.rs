use super::turtle_data::{Twist, Vector3};

pub struct TurtleControl {}

impl TurtleControl {
  pub fn move_forward() -> Twist {
    Twist {
      linear: Vector3 {
        x: 2.,
        y: 0.,
        z: 0.,
      },
      angular: Vector3 {
        x: 0.,
        y: 0.,
        z: 0.,
      },
    }
  }

  pub fn move_backward() -> Twist {
    Twist {
      linear: Vector3 {
        x: -2.,
        y: 0.,
        z: 0.,
      },
      angular: Vector3 {
        x: 0.,
        y: 0.,
        z: 0.,
      },
    }
  }

  pub fn rotate_left() -> Twist {
    Twist {
      linear: Vector3 {
        x: 0.,
        y: 0.,
        z: 0.,
      },
      angular: Vector3 {
        x: 0.,
        y: 0.,
        z: 2.,
      },
    }
  }

  pub fn rotate_right() -> Twist {
    Twist {
      linear: Vector3 {
        x: 0.,
        y: 0.,
        z: 0.,
      },
      angular: Vector3 {
        x: 0.,
        y: 0.,
        z: -2.,
      },
    }
  }
}
