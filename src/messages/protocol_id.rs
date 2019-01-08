use crate::common::validity_trait::Validity;
use speedy::{Context, Readable, Reader, Writable, Writer};

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct ProtocolId_t {
    protocol_id: [char; 4],
}

impl ProtocolId_t {
    pub const PROTOCOL_RTPS: ProtocolId_t = ProtocolId_t {
        protocol_id: ['R', 'T', 'P', 'S'],
    };
}

impl Default for ProtocolId_t {
    fn default() -> Self {
        ProtocolId_t::PROTOCOL_RTPS
    }
}

impl Validity for ProtocolId_t {
    fn valid(&self) -> bool {
        *self == ProtocolId_t::PROTOCOL_RTPS
    }
}

impl<'a, C: Context> Readable<'a, C> for ProtocolId_t {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, std::io::Error> {
        let mut protocol_id = ProtocolId_t::default();
        for i in 0..protocol_id.protocol_id.len() {
            protocol_id.protocol_id[i] = reader.read_u8()? as char;
        }
        Ok(protocol_id)
    }

    #[inline]
    fn minimum_bytes_needed() -> usize {
        4
    }
}

impl<C: Context> Writable<C> for ProtocolId_t {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(
        &'a self,
        writer: &mut T,
    ) -> Result<(), std::io::Error> {
        for elem in &self.protocol_id {
            writer.write_u8(*elem as u8)?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use speedy::Endianness;

    #[test]
    fn validity() {
        let protocol_id = ProtocolId_t::PROTOCOL_RTPS;
        assert!(protocol_id.valid());
        let protocol_id = ProtocolId_t {
            protocol_id: ['S', 'P', 'T', 'R'],
        };
        assert!(!protocol_id.valid());
    }

    #[test]
    fn minimum_bytes_needed() {
        assert_eq!(
            4,
            <ProtocolId_t as Readable<Endianness>>::minimum_bytes_needed()
        );
    }

    serialization_test!( type = ProtocolId_t,
    {
        protocol_rtps,
        ProtocolId_t::PROTOCOL_RTPS,
        le = [0x52, 0x54, 0x50, 0x53],
        be = [0x52, 0x54, 0x50, 0x53]
    });
}
