use speedy::{Context, Readable, Writable, Writer};

use crate::{
  messages::submessages::submessage_elements::parameter::Parameter,
  structure::parameter_id::ParameterId,
};

/// ParameterList is used as part of several messages to encapsulate
/// QoS parameters that may affect the interpretation of the message.
/// The encapsulation of the parameters follows a mechanism that allows
/// extensions to the QoS without breaking backwards compatibility.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct ParameterList {
  pub parameters: Vec<Parameter>,
}

impl ParameterList {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn is_empty(&self) -> bool {
    self.parameters.is_empty()
  }
}

const SENTINEL: u32 = 0x00000001;

impl<C: Context> Writable<C> for ParameterList {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    for param in &self.parameters {
      writer.write_value(param)?;
    }

    writer.write_u32(SENTINEL)?;

    Ok(())
  }
}

impl<'a, C: Context> Readable<'a, C> for ParameterList {
  #[inline]
  fn read_from<R: speedy::Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut parameters = Self::default();

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
      });
    }
  }
}
