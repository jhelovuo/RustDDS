use crate::{
  messages::submessages::submessage_elements::parameter::Parameter,
};
use speedy::{Readable, Writable, Context, Writer};

/// ParameterList is used as part of several messages to encapsulate
/// QoS parameters that may affect the interpretation of the message.
/// The encapsulation of the parameters follows a mechanism that allows
/// extensions to the QoS without breaking backwards compatibility.
#[derive(Debug, PartialEq, Readable, Clone)]
pub struct ParameterList {
  pub parameters: Vec<Parameter>,
}

impl ParameterList {
  pub fn new() -> ParameterList {
    ParameterList {
      parameters: Vec::new(),
    }
  }
}

/// The PID_PAD is used to enforce alignment of the parameter
/// that follows and its length can be anything (as long as it is a multiple of
/// 4)
pub const PID_PAD: u16 = 0x00;
const SENTINEL: u32 = 0x00000001;

impl<C: Context> Writable<C> for ParameterList {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    for param in self.parameters.iter() {
      writer.write_value(param)?;
    }

    writer.write_u32(SENTINEL)?;

    Ok(())
  }
}
