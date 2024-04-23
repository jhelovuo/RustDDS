pub(crate) mod cdr_adapters;
pub(crate) mod pl_cdr_adapters;
pub(crate) mod speedy_pl_cdr_helpers;

mod representation_identifier;

// Most of the CDR encoding/decoding comes from this external crate
pub use cdr_encoding::{
  from_bytes, to_vec, to_writer, CdrDeserializer, CdrSerializer, Error, Result,
};
pub use cdr_adapters::{CDRDeserializerAdapter, CDRSerializerAdapter, CdrDeserializeSeedDecoder};
pub use representation_identifier::RepresentationIdentifier;
