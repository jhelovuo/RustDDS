pub(crate) mod builtin_data_deserializer;
pub(crate) mod builtin_data_serializer;
pub(crate) mod cdr_deserializer;
pub(crate) mod cdr_serializer;
pub(crate) mod error;
pub(crate) mod pl_cdr_deserializer;
pub(crate) mod visitors;

pub(crate) mod message;
pub(crate) mod submessage;

// crate exports
pub(crate) use message::*;
pub(crate) use submessage::*;
// public exports
pub use cdr_serializer::{CDRSerializerAdapter, CdrSerializer};
pub use cdr_deserializer::{CDRDeserializerAdapter, CdrDeserializer};
pub use byteorder::{BigEndian, LittleEndian};

pub use crate::dds::traits::serde_adapters::{no_key, with_key};
