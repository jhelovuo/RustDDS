pub(crate) mod speedy_pl_cdr_helpers;

pub(crate) mod cdr_deserializer;
pub(crate) mod cdr_serializer;
mod error;
mod representation_identifier;

pub(crate) mod cdr_adapters;
pub(crate) mod pl_cdr_adapters;

// public exports
pub use cdr_serializer::{to_vec, to_vec_with_size_hint, to_writer, CdrSerializer};
pub use cdr_deserializer::{from_bytes, CdrDeserializer};
pub use cdr_adapters::{CDRDeserializerAdapter, CDRSerializerAdapter, CdrDeserializeSeedDecoder};
pub use error::{Error, Result};
pub use representation_identifier::RepresentationIdentifier;
