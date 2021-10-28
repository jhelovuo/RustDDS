//use crate::structure::guid::{GUID, /*EntityId, GuidPrefix*/ };
use crate::structure::time::Timestamp;
use crate::structure::sequence_number::SequenceNumber;
use crate::structure::cache_change::ChangeKind;
use crate::messages::submessages::submessages::*;
use crate::dds::ddsdata::DDSData;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;

use bit_vec::BitVec;
use enumflags2::BitFlags;
use bytes::BytesMut;
use std::collections::BTreeMap;
use std::convert::{From, TryInto};
use std::fmt;

#[allow(unused_imports)]
use log::{debug, error, warn, info, trace};

// This is for the assembly of a single object
struct AssemblyBuffer {
  buffer_bytes: BytesMut,
  fragment_count: usize,
  received_bitmap: BitVec,

  created_time: Timestamp,
  modified_time: Timestamp,
}

impl AssemblyBuffer {
  pub fn new(data_size: u32, fragment_size: u16) -> AssemblyBuffer {
    // TODO: Check that fragment size <= data_size
    // TODO: Check that fragment_size is not zero
    let data_size: usize = data_size.try_into().unwrap();
    // we have unwrap here, but it will succeed as long as usize >= u32

    let mut buffer_bytes = BytesMut::with_capacity(data_size);
    buffer_bytes.resize(data_size, 0); //TODO: Can we replace this with faster (and unsafer) .set_len and live with uninitialized data?

    let frag_size = usize::from(fragment_size);
    // fragment count formula from RTPS spec v2.5 Section 8.3.8.3.5
    let fragment_count = (data_size / frag_size) + (if data_size % frag_size != 0 { 1 } else { 0 });
    let now = Timestamp::now();

    AssemblyBuffer {
      buffer_bytes,
      fragment_count,
      received_bitmap: BitVec::from_elem(fragment_count, false),
      created_time: now,
      modified_time: now,
    }
  }

  pub fn insert_frags(&mut self, datafrag: DataFrag, frag_size: u16) {
    // TODO: Sanity checks? E.g. datafrag.fragment_size == frag_size
    let frag_size = usize::from(frag_size);
    let frags_in_subm = usize::from(datafrag.fragments_in_submessage);
    let start_frag_from_0: usize = u32::from(datafrag.fragment_starting_num)
      .try_into()
      .unwrap();

    // unwrap: u32 should fit into usize
    let from_byte = (start_frag_from_0 - 1) * frag_size;
    let to_before_byte: usize = from_byte + (frags_in_subm * frag_size);

    self.buffer_bytes.as_mut()[from_byte..to_before_byte]
      .copy_from_slice(&datafrag.serialized_payload.value);

    for f in from_byte..to_before_byte {
      self.received_bitmap.set(f, true);
    }
    self.modified_time = Timestamp::now();
  }

  pub fn is_complete(&self) -> bool {
    self.received_bitmap.all() // return if all are received
  }
}

// Assembles fragments from a single (remote) Writer
// So there is only one sequence of SNs
pub(crate) struct FragmentAssembler {
  fragment_size: u16, // number of bytes per fragment. Each writer must select one constant value.
  assembly_buffers: BTreeMap<SequenceNumber, AssemblyBuffer>,
}

impl fmt::Debug for FragmentAssembler {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("FragmentAssembler - fields omitted")
      // insert field printing here, if you really need it.
      .finish()
  }
}

impl FragmentAssembler {
  pub fn new(fragment_size: u16) -> FragmentAssembler {
    FragmentAssembler {
      fragment_size,
      assembly_buffers: BTreeMap::new(),
    }
  }

  // Returns completed DDSData, when complete, and disposes the assembly buffer.
  pub fn new_datafrag(
    &mut self,
    datafrag: DataFrag,
    flags: BitFlags<DATAFRAG_Flags>,
  ) -> Option<DDSData> {
    let rep_id = datafrag.serialized_payload.representation_identifier;
    let writer_sn = datafrag.writer_sn;

    let abuf = self
      .assembly_buffers
      .entry(datafrag.writer_sn)
      .or_insert_with(|| AssemblyBuffer::new(datafrag.data_size, datafrag.fragment_size));

    abuf.insert_frags(datafrag, self.fragment_size);

    if abuf.is_complete() {
      if let Some(abuf) = self.assembly_buffers.remove(&writer_sn) {
        // Return what we have assembled.
        let ser_data_or_key = SerializedPayload::new(rep_id, abuf.buffer_bytes.to_vec());
        let ddsdata = if flags.contains(DATAFRAG_Flags::Key) {
          DDSData::new_disposed_by_key(ChangeKind::NotAliveDisposed, ser_data_or_key)
        } else {
          // it is data
          DDSData::new(ser_data_or_key)
        };
        Some(ddsdata) // completed data from fragments
      } else {
        error!("Assembly buffer mysteriously lost");
        None
      }
    } else {
      None
    }
  }
}
