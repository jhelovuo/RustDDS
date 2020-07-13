use crate::messages::submessages::submessage_elements::parameter::Parameter;
use speedy::{Context, Readable, Reader, Writable, Writer};

/// ParameterList is used as part of several messages to encapsulate
/// QoS parameters that may affect the interpretation of the message.
/// The encapsulation of the parameters follows a mechanism that allows
/// extensions to the QoS without breaking backwards compatibility.
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct ParameterList {
  pub parameters: Vec<Parameter>,
}

impl ParameterList {
  pub fn new() -> ParameterList {
    ParameterList {parameters: Vec::new()}
  }
}

/// The PID_PAD is used to enforce alignment of the parameter
/// that follows and its length can be anything (as long as it is a multiple of
/// 4)
pub const PID_PAD: u16 = 0x00;

/// The PID_SENTINEL is used to terminate
/// the parameter list and its length is ignore
pub const PID_SENTINEL: u16 = 0x01;
