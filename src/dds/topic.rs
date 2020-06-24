//use std::time::Duration;

use crate::dds::participant::*;
//use crate::dds::key::*;
use crate::dds::typedesc::*;
use crate::dds::qos::*;

pub trait TopicDescription {
  fn get_participant(&self) -> &DomainParticipant;
  fn get_type(&self) -> TypeDesc; // This replaces get_type_name() from spec
  fn get_name(&self) -> &str;
}


pub struct Topic<'a> {
  my_domainparticipant: &'a DomainParticipant,
  my_name: String,
  my_typedesc: TypeDesc,
  my_qos_policies: QosPolicies,
}

impl<'a> TopicDescription for Topic<'a> {
  fn get_participant(&self) -> &DomainParticipant {
    self.my_domainparticipant
  }

  fn get_type(&self) -> TypeDesc {
    self.my_typedesc.clone()
  }

  fn get_name(&self) -> &str {
    &self.my_name
  }
}


impl<'a> Topic<'a> {}
