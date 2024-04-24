mod cdr_adapters;

pub(crate) mod pl_cdr_adapters;
pub(crate) mod speedy_pl_cdr_helpers;

mod representation_identifier;

// Most of the CDR encoding/decoding comes from this external crate
pub use cdr_encoding::{
  from_bytes, to_vec, to_writer, CdrDeserializer, CdrSerializer, Error, Result,
};
// Export some parts of inner modules
pub use cdr_adapters::{
  deserialize_from_cdr_with_decoder_and_rep_id, deserialize_from_cdr_with_rep_id,
  to_writer_with_rep_id, CDRDeserializerAdapter, CDRSerializerAdapter, CdrDeserializeSeedDecoder,
};
pub use representation_identifier::RepresentationIdentifier;
