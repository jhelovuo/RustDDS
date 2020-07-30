use speedy::{Context, Readable, Reader, Writable, Writer};
use uuid::Uuid;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct GuidPrefix {
  pub entityKey: [u8; 12],
}

impl GuidPrefix {
  pub const GUIDPREFIX_UNKNOWN: GuidPrefix = GuidPrefix {
    entityKey: [0x00; 12],
  };

  pub fn new(prefix: Vec<u8>) -> GuidPrefix {
    let mut pr: [u8; 12] = [0; 12];
    for (ix, &data) in prefix.iter().enumerate() {
      if ix >= 12 {
        break;
      }
      pr[ix] = data
    }
    GuidPrefix { entityKey: pr }
  }
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

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct EntityId {
  entityKey: [u8; 3],
  entityKind: u8,
}

impl EntityId {
  pub const ENTITYID_UNKNOWN: EntityId = EntityId {
    entityKey: [0x00; 3],
    entityKind: 0x00,
  };
  pub const ENTITYID_PARTICIPANT: EntityId = EntityId {
    entityKey: [0x00, 0x00, 0x01],
    entityKind: 0xC1,
  };
  pub const ENTITYID_SEDP_BUILTIN_TOPIC_WRITER: EntityId = EntityId {
    entityKey: [0x00, 0x00, 0x02],
    entityKind: 0xC2,
  };
  pub const ENTITYID_SEDP_BUILTIN_TOPIC_READER: EntityId = EntityId {
    entityKey: [0x00, 0x00, 0x02],
    entityKind: 0xC7,
  };
  pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER: EntityId = EntityId {
    entityKey: [0x00, 0x00, 0x03],
    entityKind: 0xC2,
  };
  pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER: EntityId = EntityId {
    entityKey: [0x00, 0x00, 0x03],
    entityKind: 0xC7,
  };
  pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER: EntityId = EntityId {
    entityKey: [0x00, 0x00, 0x04],
    entityKind: 0xC2,
  };
  pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER: EntityId = EntityId {
    entityKey: [0x00, 0x00, 0x04],
    entityKind: 0xC7,
  };
  pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER: EntityId = EntityId {
    entityKey: [0x00, 0x01, 0x00],
    entityKind: 0xC2,
  };
  pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER: EntityId = EntityId {
    entityKey: [0x00, 0x01, 0x00],
    entityKind: 0xC7,
  };
  pub const ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER: EntityId = EntityId {
    entityKey: [0x00, 0x02, 0x00],
    entityKind: 0xC2,
  };
  pub const ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER: EntityId = EntityId {
    entityKey: [0x00, 0x02, 0x00],
    entityKind: 0xC7,
  };

  pub fn createCustomEntityID(customEntityKey: [u8; 3], customEntityKind: u8) -> EntityId {
    EntityId {
      entityKey: customEntityKey,
      entityKind: customEntityKind,
    }
  }

  pub fn as_usize(self) -> usize {
    let x1 = self.entityKey[0] as usize;
    let x2 = self.entityKey[1] as usize;
    let x3 = self.entityKey[2] as usize;
    let x4 = self.entityKind as usize;

    x1 * 10 ^ 9 + x2 * 10 ^ 6 + x3 * 10 ^ 3 + x4
  }
}

impl Default for EntityId {
  fn default() -> EntityId {
    EntityId::ENTITYID_UNKNOWN
  }
}

impl<'a, C: Context> Readable<'a, C> for EntityId {
  #[inline]
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let entityKey = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
    let entityKind = reader.read_u8()?;
    Ok(EntityId {
      entityKey,
      entityKind,
    })
  }
}

impl<C: Context> Writable<C> for EntityId {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    for elem in &self.entityKey {
      writer.write_u8(*elem)?
    }
    writer.write_u8(self.entityKind)
  }
}

#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq, Ord, Eq, Readable, Writable, Hash)]
pub struct GUID {
  pub guidPrefix: GuidPrefix,
  pub entityId: EntityId,
}

impl GUID {
  pub const GUID_UNKNOWN: GUID = GUID {
    guidPrefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
    entityId: EntityId::ENTITYID_UNKNOWN,
  };

  /// Generates new GUID for Participant
  pub fn new() -> GUID {
    let guid = Uuid::new_v4();
    GUID {
      guidPrefix: GuidPrefix::new(guid.as_bytes().to_vec()),
      entityId: EntityId::ENTITYID_PARTICIPANT,
    }
  }

  /// Generates GUID for specific entityId from current prefix
  pub fn from_prefix(self, entity_id: EntityId) -> GUID {
    GUID {
      guidPrefix: self.guidPrefix.clone(),
      entityId: entity_id,
    }
  }

  pub fn new_with_prefix_and_id(prefix: GuidPrefix, entity_id: EntityId) -> GUID {
    GUID {
      guidPrefix: prefix,
      entityId: entity_id,
    }
  }
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

  serialization_test!( type = EntityId,
    {
        entity_unknown,
        EntityId::ENTITYID_UNKNOWN,
        le = [0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00]
    },
    {
        entity_default,
        EntityId::default(),
        le = [0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00]
    },
    {
        entity_participant,
        EntityId::ENTITYID_PARTICIPANT,
        le = [0x00, 0x00, 0x01, 0xC1],
        be = [0x00, 0x00, 0x01, 0xC1]
    },
    {
        entity_sedp_builtin_topic_writer,
        EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER,
        le = [0x00, 0x00, 0x02, 0xC2],
        be = [0x00, 0x00, 0x02, 0xC2]
    },
    {
        entity_sedp_builtin_topic_reader,
        EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER,
        le = [0x00, 0x00, 0x02, 0xC7],
        be = [0x00, 0x00, 0x02, 0xC7]
    },
    {
        entity_sedp_builtin_publications_writer,
        EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
        le = [0x00, 0x00, 0x03, 0xC2],
        be = [0x00, 0x00, 0x03, 0xC2]
    },
    {
        entity_sedp_builtin_publications_reader,
        EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
        le = [0x00, 0x00, 0x03, 0xC7],
        be = [0x00, 0x00, 0x03, 0xC7]
    },
    {
        entity_sedp_builtin_subscriptions_writer,
        EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER,
        le = [0x00, 0x00, 0x04, 0xC2],
        be = [0x00, 0x00, 0x04, 0xC2]
    },
    {
        entity_sedp_builtin_subscriptions_reader,
        EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER,
        le = [0x00, 0x00, 0x04, 0xC7],
        be = [0x00, 0x00, 0x04, 0xC7]
    },
    {
        entity_spdp_builtin_participant_writer,
        EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
        le = [0x00, 0x01, 0x00, 0xC2],
        be = [0x00, 0x01, 0x00, 0xC2]
    },
    {
        entity_spdp_builtin_participant_reader,
        EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
        le = [0x00, 0x01, 0x00, 0xC7],
        be = [0x00, 0x01, 0x00, 0xC7]
    },
    {
        entity_p2p_builtin_participant_message_writer,
        EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
        le = [0x00, 0x02, 0x00, 0xC2],
        be = [0x00, 0x02, 0x00, 0xC2]
    },
    {
        entity_p2p_builtin_participant_message_reader,
        EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER,
        le = [0x00, 0x02, 0x00, 0xC7],
        be = [0x00, 0x02, 0x00, 0xC7]
    }
  );

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
