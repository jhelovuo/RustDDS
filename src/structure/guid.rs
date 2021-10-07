use speedy::{Context, Readable, Reader, Writable, Writer};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::hash::Hash;
use std::ops::RangeBounds;

use mio::Token;
use log::warn;

use static_assertions as sa;

use super::parameter_id::ParameterId;
use crate::dds::traits::key::Key;

/// DDS/RTPS Participant GuidPrefix
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct GuidPrefix {
  pub entity_key: [u8; 12],
}

impl GuidPrefix {
  pub const GUIDPREFIX_UNKNOWN: GuidPrefix = GuidPrefix {
    entity_key: [0x00; 12],
  };

  pub fn new(prefix: &[u8]) -> GuidPrefix {
    let mut pr: [u8; 12] = [0; 12];
    for (ix, data) in prefix.iter().enumerate() {
      if ix >= 12 {
        break;
      }
      pr[ix] = *data
    }
    GuidPrefix { entity_key: pr }
  }

  pub fn range(&self) -> impl RangeBounds<GUID> {
    GUID::new(*self,EntityId::MIN)
    ..=
    GUID::new(*self,EntityId::MAX)
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
    for i in 0..guid_prefix.entity_key.len() {
      guid_prefix.entity_key[i] = reader.read_u8()?;
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
    for elem in &self.entity_key {
      writer.write_u8(*elem)?
    }
    Ok(())
  }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct EntityKind(u8);

impl EntityKind {
  // constants from RTPS spec Table 9.1
  pub const UNKNOWN_USER_DEFINED : EntityKind = EntityKind(0x00);
  //pub const PARTICIPANT_USER_DEFINED : EntityKind = EntityKind(0x01);
  // User-defined participants do not exist by definition.
  pub const WRITER_WITH_KEY_USER_DEFINED : EntityKind = EntityKind(0x02);
  pub const WRITER_NO_KEY_USER_DEFINED : EntityKind = EntityKind(0x03);
  pub const READER_NO_KEY_USER_DEFINED : EntityKind = EntityKind(0x04);
  pub const READER_WITH_KEY_USER_DEFINED : EntityKind = EntityKind(0x07);
  pub const WRITER_GROUP_USER_DEFINED : EntityKind = EntityKind(0x08);
  pub const READER_GROUP_USER_DEFINED : EntityKind = EntityKind(0x09);

  pub const UNKNOWN_BUILT_IN : EntityKind = EntityKind(0xC0);
  pub const PARTICIPANT_BUILT_IN : EntityKind = EntityKind(0xC1);
  pub const WRITER_WITH_KEY_BUILT_IN : EntityKind = EntityKind(0xC2);
  pub const WRITER_NO_KEY_BUILT_IN : EntityKind = EntityKind(0xC3);
  pub const READER_NO_KEY_BUILT_IN : EntityKind = EntityKind(0xC4);
  pub const READER_WITH_KEY_BUILT_IN : EntityKind = EntityKind(0xC7);
  pub const WRITER_GROUP_BUILT_IN : EntityKind = EntityKind(0xC8);
  pub const READER_GROUP_BUILT_IN : EntityKind = EntityKind(0xC9);

  pub const MIN : EntityKind = EntityKind(0x00);
  pub const MAX : EntityKind = EntityKind(0xFF);

  // We will encode polling tokens as EntityId, containing an EntityKind
  // The upper nibble of EntityKind will distinguish between different
  // poll tokens:
  // 0 = user-defined entity
  // 1 
  // 2 = user-defined alt token (timers etc)
  // 3 
  // 4 = fixed poll tokens (not entity-specific)
  pub const POLL_TOKEN_BASE : usize = 0x40;
  // 5 = fixed poll tokens continued
  // 6 = fixed poll tokens continued
  // 7 = fixed poll tokens continued
  // 8
  // 9
  // A
  // B
  // C = built-in entity
  // D
  // E = built-in alt token
  // F

  pub fn is_reader(&self) -> bool {
    let e = self.0 & 0x0F;
    e == 0x04 || e == 0x07 || e == 0x09
  }

  pub fn is_writer(&self) -> bool {
    let e = self.0 & 0x0F;
    e == 0x02 || e == 0x03 || e == 0x08
  }

}

impl From<u8> for EntityKind {
  fn from(b: u8) -> EntityKind {
    EntityKind(b)
  }
}

impl From<EntityKind> for u8 {
  fn from(ek: EntityKind) -> u8 {
    ek.0
  }
}

/// RTPS EntityId
/// See RTPS spec section 8.2.4 , 8.3.5.1 and 9.3.1.2
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId {
  pub entity_key: [u8; 3],
  pub entity_kind: EntityKind,
}

// We are going to pack 32 bits of payload into an usize, or ultimately
// into a mio::Token, so we need it to be large enough.
sa::const_assert!( std::mem::size_of::<usize>() >= std::mem::size_of::<u32>() );

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub enum TokenDecode {
  Entity( EntityId ),
  AltEntity( EntityId ),
  FixedToken( Token )
}


impl EntityId {
  pub const ENTITYID_UNKNOWN: EntityId = EntityId {
    entity_key: [0x00; 3],
    entity_kind: EntityKind::UNKNOWN_USER_DEFINED,
  };
  pub const ENTITYID_PARTICIPANT: EntityId = EntityId {
    entity_key: [0x00, 0x00, 0x01],
    entity_kind: EntityKind::PARTICIPANT_BUILT_IN,
  };
  pub const ENTITYID_SEDP_BUILTIN_TOPIC_WRITER: EntityId = EntityId {
    entity_key: [0x00, 0x00, 0x02],
    entity_kind: EntityKind::WRITER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_SEDP_BUILTIN_TOPIC_READER: EntityId = EntityId {
    entity_key: [0x00, 0x00, 0x02],
    entity_kind: EntityKind::READER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER: EntityId = EntityId {
    entity_key: [0x00, 0x00, 0x03],
    entity_kind: EntityKind::WRITER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER: EntityId = EntityId {
    entity_key: [0x00, 0x00, 0x03],
    entity_kind: EntityKind::READER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER: EntityId = EntityId {
    entity_key: [0x00, 0x00, 0x04],
    entity_kind: EntityKind::WRITER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER: EntityId = EntityId {
    entity_key: [0x00, 0x00, 0x04],
    entity_kind: EntityKind::READER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER: EntityId = EntityId {
    entity_key: [0x00, 0x01, 0x00],
    entity_kind: EntityKind::WRITER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER: EntityId = EntityId {
    entity_key: [0x00, 0x01, 0x00],
    entity_kind: EntityKind::READER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER: EntityId = EntityId {
    entity_key: [0x00, 0x02, 0x00],
    entity_kind: EntityKind::WRITER_WITH_KEY_BUILT_IN,
  };
  pub const ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER: EntityId = EntityId {
    entity_key: [0x00, 0x02, 0x00],
    entity_kind: EntityKind::READER_WITH_KEY_BUILT_IN,
  };


  pub const MIN: EntityId = EntityId {
    entity_key: [0x00; 3],
    entity_kind: EntityKind::MIN,
  };
  pub const MAX: EntityId = EntityId {
    entity_key: [0xFF, 0xFF, 0xFF],
    entity_kind: EntityKind::MAX,
  };


  pub fn create_custom_entity_id(entity_key: [u8; 3], entity_kind: EntityKind) -> EntityId {
    EntityId { entity_key, entity_kind }
  }

  fn as_usize(self) -> usize {
    // Usize is generated like beacause there needs to be
    // a way to tell entity kind from usize number
    let u1 = self.entity_key[0] as u32;
    let u2 = self.entity_key[1] as u32;
    let u3 = self.entity_key[2] as u32;
    let u4 = self.entity_kind.0 as u32;

    // This is essentially big-endian encoding
    // The type coercion will always succeed, because we have
    // above a static assert that usize is at least 32-bit
    ((u1 << 24) | (u2 << 16) | (u3 << 8) | u4) as usize
  }

  /// Use this only with usize generated with EntityID::as_usize function.!!!
  fn from_usize(number: usize) -> EntityId {
    let u4 = (number & 0xFF) as u8;
    let u3 = ((number >> 8) & 0xFF) as u8;
    let u2 = ((number >> 16) & 0xFF) as u8;
    let u1 = ((number >> 24) & 0xFF) as u8;

    let result =
      EntityId {
        entity_key: [u1 , u2 , u3 ],
        entity_kind: EntityKind::from( u4 )
      };

    // check sanity, as the result sohould be
    let kind_kind = u4 & (0xC0 | 0x10);
    if kind_kind == 0xC0 || kind_kind == 0x00 {
      // this is ok, all normal
    } else {
      warn!("EntityId::from_usize tried to decode 0x{:x?}",number)
    }

    result
  }

  pub fn as_token(self) -> Token {
    let u = self.as_usize();
    assert_eq!( u & !0x20 , u ); // check bit 5 is zero 
    Token( u )
  }

  pub fn as_alt_token(self) -> Token {
    Token( self.as_usize() | 0x20 ) // set bit 5
  }

  pub fn from_token(t : Token) -> TokenDecode {
    match (t.0 & 0xF0) as u8 {
      0x00 | 0xC0 => 
        TokenDecode::Entity( EntityId::from_usize( t.0 ) ) ,
      0x20 | 0xE0 => 
        TokenDecode::AltEntity( EntityId::from_usize( t.0 & !0x20 )) ,
      0x40 | 0x50 | 0x60 | 0x70 =>
        TokenDecode::FixedToken( t ) ,
      _other => {
        warn!("EntityId::from_token tried to decode 0x{:x?}",t.0);
        TokenDecode::FixedToken( t )
      }
    }
  }


  pub fn kind(self) -> EntityKind {
    self.entity_kind
  }

  pub fn set_kind(&mut self, entity_kind: EntityKind) {
    self.entity_kind = entity_kind;
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
    let entity_key = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
    let entity_kind = EntityKind(reader.read_u8()?);
    Ok(EntityId {
      entity_key,
      entity_kind,
    })
  }
}

impl<C: Context> Writable<C> for EntityId {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    for elem in &self.entity_key {
      writer.write_u8(*elem)?
    }
    writer.write_u8(self.entity_kind.0)
  }
}

/// DDS/RTPS GUID
#[derive(
  Copy,
  Clone,
  Debug,
  Default,
  PartialOrd,
  PartialEq,
  Ord,
  Eq,
  Readable,
  Writable,
  Hash,
  Serialize,
  Deserialize,
)]
pub struct GUID {
  // Note: It is important to have guid_prefix first, so that derive'd Ord trait
  // will produce ordering, where GUIDs with same GuidPrefix are grouped
  // together.
  pub guid_prefix: GuidPrefix, 
  pub entity_id: EntityId,
}

impl GUID {
  pub const GUID_UNKNOWN: GUID = GUID {
    guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
    entity_id: EntityId::ENTITYID_UNKNOWN,
  };

  // basic constructor from components
  pub fn new(prefix: GuidPrefix, entity_id: EntityId) -> GUID {
    GUID::new_with_prefix_and_id(prefix,entity_id)
  }

  /// Generates new GUID for Participant when `guid_prefix` is random
  pub fn new_particiapnt_guid() -> GUID {
    let guid = Uuid::new_v4();
    GUID {
      guid_prefix: GuidPrefix::new(guid.as_bytes()),
      entity_id: EntityId::ENTITYID_PARTICIPANT,
    }
  }

  pub fn dummy_test_guid(entity_kind: EntityKind) -> GUID {
    GUID {
      guid_prefix: GuidPrefix::new(b"FakeTestGUID"),
      entity_id: EntityId {
        entity_key: [1,2,3] ,
        entity_kind,
      },
    }
  }

  /// Generates GUID for specific entity_id from current prefix
  pub fn from_prefix(self, entity_id: EntityId) -> GUID {
    GUID {
      guid_prefix: self.guid_prefix,
      entity_id,
    }
  }

  /// Creates GUID from known values
  pub fn new_with_prefix_and_id(prefix: GuidPrefix, entity_id: EntityId) -> GUID {
    GUID {
      guid_prefix: prefix,
      entity_id,
    }
  }

  pub fn as_usize(&self) -> usize {
    self.entity_id.as_usize()
  }

}

impl Key for GUID {}

#[derive(Serialize, Deserialize)]
pub(crate) struct GUIDData {
  parameter_id: ParameterId,
  parameter_length: u16,
  guid: GUID,
}

impl GUIDData {
  pub fn from(guid: GUID, parameter_id: ParameterId) -> GUIDData {
    GUIDData {
      parameter_id,
      parameter_length: 16,
      guid,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use speedy::Endianness;
  use mio::Token;
  use log::info;

  #[test]
  fn convert_entity_id_to_token_and_back() {
    let e = EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER;
    let _t = Token(e.as_usize());
    info!("{:?}", e.as_usize());
    let entity = EntityId::from_usize(e.as_usize());
    assert_eq!(e, entity);

    let e2 = EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER;
    let entity2 = EntityId::from_usize(e2.as_usize());
    assert_eq!(e2, entity2);

    let e3 = EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER;
    let entity3 = EntityId::from_usize(e3.as_usize());
    assert_eq!(e3, entity3);

    let e4 = EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER;
    let entity4 = EntityId::from_usize(e4.as_usize());
    assert_eq!(e4, entity4);

    let e5 = EntityId::ENTITYID_UNKNOWN;
    let entity5 = EntityId::from_usize(e5.as_usize());
    assert_eq!(e5, entity5);

    let e6 = EntityId::create_custom_entity_id([12u8, 255u8, 0u8], EntityKind(254u8) );
    let entity6 = EntityId::from_usize(e6.as_usize());
    assert_eq!(e6, entity6);
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
          entity_key: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
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
        entity_id: EntityId::ENTITYID_UNKNOWN,
        guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN
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
              entity_id: EntityId::ENTITYID_PARTICIPANT,
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
