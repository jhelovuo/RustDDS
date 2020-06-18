use speedy::{Context, Readable, Reader, Writable, Writer};

#[derive(Debug, PartialEq, Eq)]
pub struct VendorId {
  pub vendorId: [u8; 2],
}

impl VendorId {
  pub const VENDOR_UNKNOWN: VendorId = VendorId {
    vendorId: [0x00; 2],
  };
}

impl Default for VendorId {
  fn default() -> Self {
    VendorId::VENDOR_UNKNOWN
  }
}

impl<'a, C: Context> Readable<'a, C> for VendorId {
  #[inline]
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut vendor_id = VendorId::default();
    for i in 0..vendor_id.vendorId.len() {
      vendor_id.vendorId[i] = reader.read_u8()?;
    }
    Ok(vendor_id)
  }

  #[inline]
  fn minimum_bytes_needed() -> usize {
    std::mem::size_of::<Self>()
  }
}

impl<C: Context> Writable<C> for VendorId {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    for elem in &self.vendorId {
      writer.write_u8(*elem)?
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use speedy::Endianness;

  #[test]
  fn minimum_bytes_needed() {
    assert_eq!(
      2,
      <VendorId as Readable<Endianness>>::minimum_bytes_needed()
    );
  }

  serialization_test!( type = VendorId,
  {
      vendor_unknown,
      VendorId::VENDOR_UNKNOWN,
      le = [0x00, 0x00],
      be = [0x00, 0x00]
  });
}
