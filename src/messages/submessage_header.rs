use crate::messages::submessage_flag::SubmessageFlag;
use crate::messages::submessage_kind::SubmessageKind;
use speedy::{Context, Endianness, Readable, Reader, Writable, Writer};

#[derive(Debug, PartialEq)]
pub struct SubmessageHeader {
    pub submessage_id: SubmessageKind,
    pub flags: SubmessageFlag,
    pub submessage_length: u16,
}

impl<'a, C: Context> Readable<'a, C> for SubmessageHeader {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, std::io::Error> {
        let submessage_id: SubmessageKind = reader.read_value()?;
        let flags: SubmessageFlag = reader.read_value()?;
        let submessage_length = match flags.endianness_flag() {
            Endianness::LittleEndian => u16::from_le_bytes([reader.read_u8()?, reader.read_u8()?]),
            Endianness::BigEndian => u16::from_be_bytes([reader.read_u8()?, reader.read_u8()?]),
        };
        Ok(SubmessageHeader {
            submessage_id,
            flags,
            submessage_length,
        })
    }

    #[inline]
    fn minimum_bytes_needed() -> usize {
        std::mem::size_of::<Self>()
    }
}

impl<C: Context> Writable<C> for SubmessageHeader {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(
        &'a self,
        writer: &mut T,
    ) -> Result<(), std::io::Error> {
        writer.write_value(&self.submessage_id)?;
        writer.write_value(&self.flags)?;

        match &self.flags.endianness_flag() {
            // matching via writer.context().endianness() panics
            speedy::Endianness::LittleEndian => {
                writer.write_u8(self.submessage_length as u8)?;
                writer.write_u8((self.submessage_length >> 8) as u8)?;
            }
            speedy::Endianness::BigEndian => {
                writer.write_u8((self.submessage_length >> 8) as u8)?;
                writer.write_u8(self.submessage_length as u8)?;
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = SubmessageHeader,
    {
        submessage_header_big_endian_flag,
        SubmessageHeader {
            submessage_id: SubmessageKind::ACKNACK,
            flags: SubmessageFlag { flags: 0x00 },
            submessage_length: 42,
        },
        le = [0x06, 0x00, 0x00, 0x2A],
        be = [0x06, 0x00, 0x00, 0x2A]
    },
    {
        submessage_header_little_endian_flag,
        SubmessageHeader {
            submessage_id: SubmessageKind::ACKNACK,
            flags: SubmessageFlag { flags: 0x01 },
            submessage_length: 42,
        },
        le = [0x06, 0x01, 0x2A, 0x00],
        be = [0x06, 0x01, 0x2A, 0x00]
    },
    {
        submessage_header_big_endian_2_bytes_length,
        SubmessageHeader {
            submessage_id: SubmessageKind::ACKNACK,
            flags: SubmessageFlag { flags: 0x00 },
            submessage_length: 258,
        },
        le = [0x06, 0x00, 0x01, 0x02],
        be = [0x06, 0x00, 0x01, 0x02]
    },
    {
        submessage_header_little_endian_2_bytes_length,
        SubmessageHeader {
            submessage_id: SubmessageKind::ACKNACK,
            flags: SubmessageFlag { flags: 0x01 },
            submessage_length: 258,
        },
        le = [0x06, 0x01, 0x02, 0x01],
        be = [0x06, 0x01, 0x02, 0x01]
    },
    {
        submessage_header_wireshark,
        SubmessageHeader {
            submessage_id: SubmessageKind::INFO_TS,
            flags: SubmessageFlag { flags: 0x01 },
            submessage_length: 8,
        },
        le = [0x09, 0x01, 0x08, 0x00],
        be = [0x09, 0x01, 0x08, 0x00]
    },
    {
        submessage_header_gap,
        SubmessageHeader {
            submessage_id: SubmessageKind::GAP,
            flags: SubmessageFlag { flags: 0x03 },
            submessage_length: 7,
        },
        le = [0x08, 0x03, 0x07, 0x00],
        be = [0x08, 0x03, 0x07, 0x00]
    });
}
