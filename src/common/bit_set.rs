use bit_set::BitSet;
use bit_vec::BitVec;
use log::debug;
use speedy::{Context, Readable, Reader, Writable, Writer};
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq)]
pub struct BitSetRef(BitSet);

impl BitSetRef {
  pub fn new() -> BitSetRef {
    BitSetRef(BitSet::with_capacity(0))
  }

  pub fn into_bit_set(self) -> BitSet {
    self.0
  }
}

impl Deref for BitSetRef {
  type Target = BitSet;

  fn deref(&self) -> &BitSet {
    &self.0
  }
}

impl DerefMut for BitSetRef {
  fn deref_mut(&mut self) -> &mut BitSet {
    &mut self.0
  }
}

impl<'a, C: Context> Readable<'a, C> for BitSetRef {
  #[inline]
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let number_of_bits = reader.read_u32()?;
    if number_of_bits == 0 {
      return Ok(BitSetRef::new());
    }

    let mut bit_vec = BitVec::with_capacity(number_of_bits as usize);

    for _ in 0..(number_of_bits / 32) {
      // read value should be directly correct
      let mut byte = reader.read_u32()?;

      // reading whoe buffer to bitvec
      while byte > 0 {
        let val = (byte & 0x80000000) > 0;
        byte = byte << 1;
        bit_vec.push(val);
      }
    }

    if number_of_bits % 32 != 0 {
      // uneven number of bits
      let mut byte = reader.read_u32()?;
      debug!("Read ack: {:x?} :: {:b}", byte, byte);
      // rotating to correct alignment
      // byte = byte.rotate_right(32 - number_of_bits % 32);

      // reading whoe buffer to bitvec
      while byte > 0 {
        let val = (byte & 0x80000000) > 0;
        byte = byte << 1;
        bit_vec.push(val);
      }
    }

    debug!("Full Bitvec: {:x?} :: {:?}", bit_vec, bit_vec);

    Ok(BitSetRef(BitSet::from_bit_vec(bit_vec)))
  }

  #[inline]
  fn minimum_bytes_needed() -> usize {
    4
  }
}

impl<C: Context> Writable<C> for BitSetRef {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    let mut number_of_bytes = 0;

    let mut values = Vec::new();
    let mut value: i32 = 0;
    for b in self.get_ref().iter() {
      number_of_bytes += 1;
      value = value << 1;
      value += if b { 1 } else { 0 };
      if number_of_bytes % 32 == 0 {
        values.push(value);
        value = 0;
      }
    }

    writer.write_u32(number_of_bytes)?;

    for val in values.iter_mut() {
      let lz = val.leading_zeros();
      let foo = val.rotate_left(lz);
      writer.write_i32(foo)?;
    }

    if values.is_empty() && number_of_bytes > 0 {
      let lz = value.leading_zeros();
      let foo = value.rotate_left(lz);
      writer.write_i32(foo)?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = BitSetRef,
  {
      bit_set_empty,
      BitSetRef::new(),
      le = [0x00, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x00]
  },
  {
      bit_set_non_zero_size,
      (|| {
          let mut set = BitSetRef::new();
          set.insert(0);
          set.insert(42);
          set.insert(7);
          set
      })(),
      le = [0x40, 0x00, 0x00, 0x00,
            0x81, 0x00, 0x00, 0x00,],
      // le = [0x40, 0x00, 0x00, 0x00,
      //       0x81, 0x00, 0x00, 0x00,
      //       0x00, 0x04, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x40,
            0x00, 0x00, 0x00, 0x81,
            0x00, 0x00, 0x04, 0x00]
  });
}
