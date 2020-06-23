use std::time::Duration;

use crate::structure::result::*;

use crate::dds::participant::*;

pub struct Publisher<'a> {
  my_domainparticipant: &'a DomainParticipant,
} 
pub struct Subscriber<'a> {
  my_domainparticipant: &'a DomainParticipant,
}


// public interface for Publisher
impl<'a> Publisher<'a> 
{
  pub fn create_datawriter() -> Result<DataWriter> {
    unimplemented!();
  }

  // delete_datawriter should not be needed. The DataWriter object itself should be deleted to accomplish this.

  // lookup datawriter: maybe not necessary? App should remember datawriters it has created.

  // Suspend and resume publications are preformance optimization methods.
  // The minimal correct implementation is to do nothing. See DDS spec 2.2.2.4.1.8 and .9
  pub fn suspend_publications(&self) -> Result<()> { Ok(()) }
  pub fn resume_publications(&self) -> Result<()> { Ok(()) }

  // coherent change set
  // In case such QoS is not supported, these should be no-ops.
  pub fn begin_coherent_changes(&self) -> Result<()> { Ok(()) }
  pub fn end_coherent_changes(&self) -> Result<()> { Ok(()) }

  // Wait for all matched reliable DataReaders acknowledge data written so far, or timeout.
  pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> Result<()> {
    unimplemented!();
  }

  // What is the use case for this? (is it useful in Rust style of programming? Should it be public?)
  pub fn get_participant(&self) -> &DomainParticipant { self.my_domainparticipant }

  // delete_contained_entities: We should not need this. Contained DataWriters should dispose themselves and notify publisher.

  

} 

