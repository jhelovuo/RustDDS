use crate::common::parameter_id::ParameterId;
use crate::message::Validity;
use speedy::{Context, Readable, Reader, Writable, Writer};
use std::io::Result;

#[derive(Debug, PartialEq)]
pub struct Parameter {
    /// Uniquely identifies the type of parameter
    parameter_id: ParameterId,
    /// Contains the CDR encapsulation of the Parameter type
    /// that corresponds to the specified parameterId
    value: Vec<u8>,
}

impl<'a, C: Context> Readable<'a, C> for Parameter {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self> {
        let parameter_id: ParameterId = reader.read_value()?;
        let length = reader.read_u16()?;
        let alignment = length % 4;

        let mut value = Vec::with_capacity((length + alignment) as usize);

        for _ in 0..(length + alignment) {
            let byte = reader.read_u8()?;
            value.push(byte);
        }

        Ok(Parameter {
            parameter_id: parameter_id,
            value: value,
        })
    }

    #[inline]
    fn minimum_bytes_needed() -> usize {
        32
    }
}

impl<C: Context> Writable<C> for Parameter {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(&'a self, writer: &mut T) -> Result<()> {
        writer.write_value(&self.parameter_id)?;

        let length = self.value.len();
        let alignment = length % 4;
        writer.write_u16((length + alignment) as u16)?;

        for byte in &self.value {
            writer.write_u8(*byte)?;
        }

        for _ in 0..alignment {
            writer.write_u8(0x00)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = Parameter,
    {
        pid_protocol_version,
        Parameter {
            parameter_id: ParameterId::PID_PROTOCOL_VERSION,
            value: vec![0x02, 0x01, 0x00, 0x00],
        },
        le = [0x15, 0x00, 0x04, 0x00,
              0x02, 0x01, 0x00, 0x00],
        be = [0x00, 0x15, 0x00, 0x04,
              0x02, 0x01, 0x00, 0x00]
    },
    {
        pid_vendor_id,
        Parameter {
            parameter_id: ParameterId::PID_VENDOR_ID,
            value: vec![0x01, 0x02, 0x03, 0x04],
        },
        le = [0x16, 0x00, 0x04, 0x00,
              0x01, 0x02, 0x03, 0x04],
        be = [0x00, 0x16, 0x00, 0x04,
              0x01, 0x02, 0x03, 0x04]
    },
    {
        pid_participant_guid,
        Parameter {
            parameter_id: ParameterId::PID_PARTICIPANT_GUID,
            value: vec![0x01, 0x0F, 0xBB, 0x1D,
                        0xDF, 0x2B, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x01, 0xC1],
        },
        le = [0x50, 0x00, 0x10, 0x00,
              0x01, 0x0F, 0xBB, 0x1D,
              0xDF, 0x2B, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x01, 0xC1],
        be = [0x00, 0x50, 0x00, 0x10,
              0x01, 0x0F, 0xBB, 0x1D,
              0xDF, 0x2B, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x01, 0xC1]
    },
    {
        pid_participant_lease_duration,
        Parameter {
            parameter_id: ParameterId::PID_PARTICIPANT_LEASE_DURATION,
            value: vec![0xFF, 0xFF, 0xFF, 0x7F,
                        0xFF, 0xFF, 0xFF, 0xFF],
        },
        le = [0x02, 0x00, 0x08, 0x00,
              0xFF, 0xFF, 0xFF, 0x7F,
              0xFF, 0xFF, 0xFF, 0xFF],
        be = [0x00, 0x02, 0x00, 0x08,
              0xFF, 0xFF, 0xFF, 0x7F,
              0xFF, 0xFF, 0xFF, 0xFF]
    });
}
