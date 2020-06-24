use std::time::Duration;

use crate::dds::participant::*;
//use crate::dds::key::*;

pub struct Topic<'a> {
  my_domainparticipant: &'a DomainParticipant,
  my_qos_policies: QosPolicies,
}

impl<'a> Topic<'a> {}
