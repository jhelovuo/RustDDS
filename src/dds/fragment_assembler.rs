use crate::structure::guid::{GUID, /*EntityId, GuidPrefix*/ };
use enumflags2::BitFlags;
use crate::messages::submessages::submessages::*;
use crate::dds::ddsdata::DDSData;


pub(crate) struct FragmentAssembler {

}

impl FragmentAssembler {
	pub fn new() -> FragmentAssembler {
		FragmentAssembler {}
	}

	// Returns compl
	pub fn new_datafrag(&mut self, writer_guid:GUID, datafrag:DataFrag, flags: BitFlags<DATAFRAG_Flags>) 
		-> Option<DDSData>
	{
		// the Data return value is placeholder for completed data
		todo!()
	}
}