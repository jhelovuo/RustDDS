use crate::{
  messages::submessages::submessage_elements::parameter::Parameter,
  structure::parameter_id::ParameterId,
};
use speedy::{Readable, Writable, Context, Writer};

/// ParameterList is used as part of several messages to encapsulate
/// QoS parameters that may affect the interpretation of the message.
/// The encapsulation of the parameters follows a mechanism that allows
/// extensions to the QoS without breaking backwards compatibility.
#[derive(Debug, PartialEq, Clone)]
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

impl<'a, C: Context> Readable<'a, C> for ParameterList {
  #[inline]
  fn read_from<R: speedy::Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut parameters = ParameterList::new();

    // loop ends in failure to read something or catching sentinel
    loop {
      let parameter_id = ParameterId::read_from(reader)?;
      let length = u16::read_from(reader)?;

      if parameter_id == ParameterId::PID_SENTINEL {
        // This is paramter list end marker.
        return Ok(parameters);
      }

      parameters.parameters.push(Parameter {
        parameter_id,
        value: reader.read_vec(length as usize)?,
      })
    }
  }
}
