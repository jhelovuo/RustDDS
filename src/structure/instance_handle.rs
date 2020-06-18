use speedy::{Context, Readable, Reader, Writable, Writer};

/// Type used to represent the identity of a data-object whose changes in value
/// are communicated by the RTPS protocol.
#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct InstanceHandle {
  pub entityKey: [u8; 16],
}

impl Default for InstanceHandle {
  fn default() -> InstanceHandle {
    InstanceHandle {
      entityKey: [0x00; 16],
    }
  }
}

impl<'a, C: Context> Readable<'a, C> for InstanceHandle {
  #[inline]
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut instance_handle = InstanceHandle::default();
    for i in 0..instance_handle.entityKey.len() {
      instance_handle.entityKey[i] = reader.read_u8()?;
    }
    Ok(instance_handle)
  }
}

impl<C: Context> Writable<C> for InstanceHandle {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    for elem in &self.entityKey {
      writer.write_u8(*elem)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = InstanceHandle,
      {
          instance_handle_default,
          InstanceHandle::default(),
          le = [0x00; 16],
          be = [0x00; 16]
      },
      {
          instance_handle_endianness_insensitive,
          InstanceHandle {
              entityKey: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                          0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
          },
          le = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
          be = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
      }
  );
}
