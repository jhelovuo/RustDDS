use speedy::{Context, Readable, Reader, Writable, Writer};
use std::convert::From;
use std::mem::size_of;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SequenceNumber_t {
    value: i64,
}

impl SequenceNumber_t {
    pub const SEQUENCENUMBER_UNKNOWN: SequenceNumber_t = SequenceNumber_t {
        value: (std::u32::MAX as i64) << 32,
    };
}

impl From<i64> for SequenceNumber_t {
    fn from(value: i64) -> Self {
        SequenceNumber_t { value }
    }
}

impl From<SequenceNumber_t> for i64 {
    fn from(sequence_number: SequenceNumber_t) -> Self {
        sequence_number.value
    }
}

impl<'a, C: Context> Readable<'a, C> for SequenceNumber_t {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, std::io::Error> {
        let high: i32 = reader.read_value()?;
        let low: u32 = reader.read_value()?;

        Ok(SequenceNumber_t {
            value: ((i64::from(high)) << 32) + i64::from(low),
        })
    }

    #[inline]
    fn minimum_bytes_needed() -> usize {
        size_of::<Self>()
    }
}

impl<C: Context> Writable<C> for SequenceNumber_t {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(
        &'a self,
        writer: &mut T,
    ) -> Result<(), std::io::Error> {
        writer.write_i32((self.value >> 32) as i32)?;
        writer.write_u32(self.value as u32)?;
        Ok(())
    }
}

impl Default for SequenceNumber_t {
    fn default() -> SequenceNumber_t {
        SequenceNumber_t { value: 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_number_starts_by_default_from_one() {
        assert_eq!(SequenceNumber_t::from(1), SequenceNumber_t::default());
    }

    serialization_test!( type = SequenceNumber_t,
    {
        sequence_number_default,
        SequenceNumber_t::default(),
        le = [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
    },
    {
        sequence_number_unknown,
        SequenceNumber_t::SEQUENCENUMBER_UNKNOWN,
        le = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00],
        be = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00]
    },
    {
        sequence_number_non_zero,
        SequenceNumber_t::from(0x0011223344556677),
        le = [0x33, 0x22, 0x11, 0x00, 0x77, 0x66, 0x55, 0x44],
        be = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]
    });
}
