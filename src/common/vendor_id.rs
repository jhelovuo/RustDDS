use speedy::{Context, Readable, Reader, Writable, Writer};
use std::io::Result;
use std::mem::size_of;

#[derive(Debug, PartialEq, Eq)]
pub struct VendorId_t {
    pub vendorId: [u8; 2],
}

pub const VENDOR_UNKNOWN: VendorId_t = VendorId_t {
    vendorId: [0x00; 2],
};

impl Default for VendorId_t {
    fn default() -> Self {
        VENDOR_UNKNOWN
    }
}

impl<'a, C: Context> Readable<'a, C> for VendorId_t {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self> {
        let mut vendor_id = VendorId_t::default();
        for i in 0..vendor_id.vendorId.len() {
            vendor_id.vendorId[i] = reader.read_u8()?;
        }
        Ok(vendor_id)
    }

    #[inline]
    fn minimum_bytes_needed() -> usize {
        size_of::<Self>()
    }
}

impl<C: Context> Writable<C> for VendorId_t {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(&'a self, writer: &mut T) -> Result<()> {
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
            <VendorId_t as Readable<Endianness>>::minimum_bytes_needed()
        );
    }

    serialization_test!( type = VendorId_t,
    {
        vendor_unknown,
        VENDOR_UNKNOWN,
        le = [0x00, 0x00],
        be = [0x00, 0x00]
    });
}
