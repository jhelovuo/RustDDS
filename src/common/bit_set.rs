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

    for _ in 0..(number_of_bits / 32) + 1 {
      // read value should be directly correct
      let byte = reader.read_u32()?.reverse_bits();
      unsafe {
        let inner = bit_vec.storage_mut();
        inner.push(byte);
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
    let number_of_bytes = self.get_ref().iter().count();
    let mut values = Vec::new();

    for &value in self.get_ref().storage() {
      values.push(value);
    }

    writer.write_u32(number_of_bytes as u32)?;

    for val in values.iter_mut() {
      let foo = val.reverse_bits();
      writer.write_u32(foo)?;
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
      le = [0x2B, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x81,
            0x00, 0x00, 0x20, 0x00],
      be = [0x00, 0x00, 0x00, 0x2B,
            0x81, 0x00, 0x00, 0x00,
            0x00, 0x20, 0x00, 0x00]
  });
}
