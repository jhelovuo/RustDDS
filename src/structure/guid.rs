use crate::structure::entity::EntityId;
use speedy::{Context, Readable, Reader, Writable, Writer};

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct GuidPrefix {
  pub entityKey: [u8; 12],
}

impl GuidPrefix {
  pub const GUIDPREFIX_UNKNOWN: GuidPrefix = GuidPrefix {
    entityKey: [0x00; 12],
  };
}

impl Default for GuidPrefix {
  fn default() -> GuidPrefix {
    GuidPrefix::GUIDPREFIX_UNKNOWN
  }
}

impl<'a, C: Context> Readable<'a, C> for GuidPrefix {
  #[inline]
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut guid_prefix = GuidPrefix::default();
    for i in 0..guid_prefix.entityKey.len() {
      guid_prefix.entityKey[i] = reader.read_u8()?;
    }
    Ok(guid_prefix)
  }

  #[inline]
  fn minimum_bytes_needed() -> usize {
    std::mem::size_of::<Self>()
  }
}

impl<C: Context> Writable<C> for GuidPrefix {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    for elem in &self.entityKey {
      writer.write_u8(*elem)?
    }
    Ok(())
  }
}

#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq, Ord, Eq, Readable, Writable)]
pub struct GUID {
  pub guidPrefix: GuidPrefix,
  pub entityId: EntityId,
}

impl GUID {
  pub const GUID_UNKNOWN: GUID = GUID {
    guidPrefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
    entityId: EntityId::ENTITYID_UNKNOWN,
  };
}

#[cfg(test)]
mod tests {
  use super::*;

  use speedy::Endianness;

  #[test]
  fn minimum_bytes_needed() {
    assert_eq!(
      12,
      <GuidPrefix as Readable<Endianness>>::minimum_bytes_needed()
    );
  }

  serialization_test!( type = GuidPrefix,
  {
      guid_prefix_unknown,
      GuidPrefix::GUIDPREFIX_UNKNOWN,
      le = [0x00; 12],
      be = [0x00; 12]
  },
  {
      guid_prefix_default,
      GuidPrefix::default(),
      le = [0x00; 12],
      be = [0x00; 12]
  },
  {
      guid_prefix_endianness_insensitive,
      GuidPrefix {
          entityKey: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                      0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]
      },
      le = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB],
      be = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]
  });

  #[test]
  fn guid_unknown_is_a_combination_of_unknown_members() {
    assert_eq!(
      GUID {
        entityId: EntityId::ENTITYID_UNKNOWN,
        guidPrefix: GuidPrefix::GUIDPREFIX_UNKNOWN
      },
      GUID::GUID_UNKNOWN
    );
  }

  serialization_test!( type = GUID,
      {
          guid_unknown,
          GUID::GUID_UNKNOWN,
          le = [0x00; 16],
          be = [0x00; 16]
      },
      {
          guid_default,
          GUID::default(),
          le = [0x00; 16],
          be = [0x00; 16]
      },
      {
          guid_entity_id_on_the_last_position,
          GUID {
              entityId: EntityId::ENTITYID_PARTICIPANT,
              ..Default::default()
          },
          le = [0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x01, 0xC1],
          be = [0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x01, 0xC1]
      }
  );
}
