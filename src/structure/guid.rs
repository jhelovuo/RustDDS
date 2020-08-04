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
    // Usize is generated like beacause there needs to be a way to tell entity kind from usize number
    let x1 = self.entityKey[0] as i64;
    let x2 = self.entityKey[1] as i64;
    let x3 = self.entityKey[2] as i64;
    let x4 = self.entityKind as i64;

    /*
    println!("{:?}", (10_i64.pow(14) + x1 * 100000000000));
    println!("{:?}", (10_i64.pow(10) + x2 * 10000000));
    println!("{:?}", (10_i64.pow(6) + x3 * 1000) );
    println!("{:?}", x4 as i64);
    */
   
    ((10_i64.pow(14) + x1 * 100000000000) + (10_i64.pow(10) + x2 * 10000000) + (10_i64.pow(6) + x3 * 1000) +  x4 as i64) as usize

  }
  
  /// Use this only with usize generated with EntityID::as_usize function.!!!
  pub fn from_usize(number: usize) -> Option<EntityId> {
    //println!("{:?}",number);
    let numberAsString = number.to_string();
    let finalIndex = numberAsString.len();
    if finalIndex != 15 {
      return None;
    }
    let kind = numberAsString[finalIndex-3..finalIndex] .parse::<u8>().unwrap();
    let thirdByte = numberAsString[finalIndex-6..finalIndex-3] .parse::<u8>().unwrap();
    let secondBute = numberAsString[finalIndex-10..finalIndex-7] .parse::<u8>().unwrap();
    let firstByte = numberAsString[finalIndex-14..finalIndex-11] .parse::<u8>().unwrap();
    let e : EntityId = EntityId {
      entityKey : [firstByte,secondBute,thirdByte],
      entityKind : kind,
    };
    return Some(e);
  }

  pub fn get_kind(self) -> u8 {
    return self.entityKind
  }

  pub fn set_kind(&mut self, entityKind : u8 ) {
    self.entityKind = entityKind;
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
  use mio::Token;

  #[test]
  fn convert_entity_id_to_token_and_back(){
    let e = EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER;
    let t = Token(e.as_usize());
    println!("{:?}", e.as_usize());
    let entity = EntityId::from_usize(e.as_usize()).unwrap();
    assert_eq!(e,entity);

    let e2 = EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER;
    let entity2 = EntityId::from_usize(e2.as_usize()).unwrap();
    assert_eq!(e2,entity2);

    let e3 = EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER;
    let entity3 = EntityId::from_usize(e3.as_usize()).unwrap();
    assert_eq!(e3,entity3);
    
    let e4 = EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER;
    let entity4 = EntityId::from_usize(e4.as_usize()).unwrap();
    assert_eq!(e4,entity4);

    let e5 = EntityId::ENTITYID_UNKNOWN;
    let entity5 = EntityId::from_usize(e5.as_usize()).unwrap();
    assert_eq!(e5,entity5);


    let e6 = EntityId::createCustomEntityID([12u8,255u8,0u8],254u8);
    let entity6 = EntityId::from_usize(e6.as_usize()).unwrap();
    assert_eq!(e6,entity6);
  }

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
