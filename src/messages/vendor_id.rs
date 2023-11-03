use std::fmt;

use speedy::{Context, Readable, Reader, Writable, Writer};

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct VendorId {
  pub vendor_id: [u8; 2],
}

impl VendorId {
  pub const VENDOR_UNKNOWN: Self = Self {
    vendor_id: [0x00, 0x00],
  };

  /// assigned by OMG DDS SIG on 2020-11-21
  pub const ATOSTEK: Self = Self {
    vendor_id: [0x01, 0x12],
  };

  pub const THIS_IMPLEMENTATION: Self = Self::ATOSTEK;

  pub fn as_bytes(&self) -> [u8; 2] {
    self.vendor_id
  }

  fn known_vendor_id_string(self) -> Option<(&'static str,&'static str)> {
    match self.vendor_id {
      // from https://www.dds-foundation.org/dds-rtps-vendor-and-product-ids/
      // on 2023-11-03
      [0x01, 0x01] => Some(("RTI Connext DDS","Real-Time Innovations, Inc. (RTI)")),
      [0x01, 0x02] => Some(("OpenSplice DDS","ADLink Ltd.")),
      [0x01, 0x03] => Some(("OpenDDS","Object Computing Inc. (OCI)")),
      [0x01, 0x04] => Some(("Mil-DDS","MilSoft")),
      [0x01, 0x05] => Some(("InterCOM","DDS  Kongsberg")),
      [0x01, 0x06] => Some(("CoreDX DDS","Twin Oaks Computing")),
      [0x01, 0x07] => Some(("Not Activ","Lakota Technical Solutions, Inc.")),
      [0x01, 0x08] => Some(("Not Active","ICOUP Consulting")),
      [0x01, 0x09] => Some(("Diamond DDS","Electronics and Telecommunication Research Institute (ETRI)")),
      [0x01, 0x0A] => Some(("RTI Connext","DDS Micro Real-Time Innovations, Inc. (RTI)")),
      [0x01, 0x0B] => Some(("Vortex Cafe","ADLink Ltd.")),
      [0x01, 0x0C] => Some(("Not Active","PrismTech Ltd.")),
      [0x01, 0x0D] => Some(("Vortex Lite","ADLink Ltd.")),
      [0x01, 0x0E] => Some(("Qeo (Not active)","Technicolor")),
      [0x01, 0x0F] => Some(("FastRTPS, FastDDS","eProsima")),
      [0x01, 0x10] => Some(("Eclipse Cyclone DDS","Eclipse Foundation")),
      [0x01, 0x11] => Some(("GurumDDS","Gurum Networks, Inc.")),
      [0x01, 0x12] => Some(("RustDDS","Atostek")),
      [0x01, 0x13] => Some(("Zhenrong Data Distribution Service (ZRDDS)","Nanjing Zhenrong Software Technology Co.")),
      [0x01, 0x14] => Some(("Dust DDS","S2E Software Systems B.V.")),
      _ => None,
    }
  }
}

impl Default for VendorId {
  fn default() -> Self {
    Self::VENDOR_UNKNOWN
  }
}

 

impl fmt::Debug for VendorId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Self::VENDOR_UNKNOWN => write!(f,"VENDOR_UNKNOWN"),
      other => match other.known_vendor_id_string() {
        Some((product,vendor)) => write!(f,"{product} / {vendor}"),
        None => write!(f,"{:x?}", other.vendor_id),
      }
    }
  }
}

impl<'a, C: Context> Readable<'a, C> for VendorId {
  #[inline]
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut vendor_id = Self::default();
    for i in 0..vendor_id.vendor_id.len() {
      vendor_id.vendor_id[i] = reader.read_u8()?;
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
    for elem in &self.vendor_id {
      writer.write_u8(*elem)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use speedy::Endianness;

  use super::*;

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
