use common::submessage_flag::{SubmessageFlag};
use common::entity_id::{EntityId_t};
use common::sequence_number::{SequenceNumberSet_t};
use common::count::{Count_t};

struct AckNack {
    
    reader_id: EntityId_t,
    writer_id: EntityId_t,
    reader_sn_state: SequenceNumberSet_t,
    count: Count_t
}
