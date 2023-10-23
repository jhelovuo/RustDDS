use speedy::{Context, Readable, Writable};

/// CryptoFooter: section 7.3.6.2.4 of the Security specification (v.
/// 1.1)
/// Should be interpreted by the plugin based on `transformation_id`
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CryptoFooter {
  pub data: Vec<u8>,
}
impl From<Vec<u8>> for CryptoFooter {
  fn from(data: Vec<u8>) -> Self {
    Self { data }
  }
}
impl From<CryptoFooter> for Vec<u8> {
  fn from(CryptoFooter { data }: CryptoFooter) -> Self {
    data
  }
}

// According to 9.5.2.5 and Wireshark, CryptoFooter is just a blob of data
// specific to plugin implementation, and the serialization does not include its
// length
impl<C: Context> Writable<C> for CryptoFooter {
  fn write_to<T: ?Sized + speedy::Writer<C>>(
    &self,
    writer: &mut T,
  ) -> Result<(), <C as Context>::Error> {
    writer.write_bytes(&self.data)
  }
}

impl<'a, C: Context> Readable<'a, C> for CryptoFooter {
  fn read_from<R: speedy::Reader<'a, C>>(reader: &mut R) -> Result<Self, <C as Context>::Error> {
    reader.read_vec_until_eof().map(Self::from)
  }
}
