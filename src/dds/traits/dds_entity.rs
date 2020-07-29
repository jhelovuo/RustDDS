use crate::dds::qos::HasQoSPolicy;

pub trait DDSEntity<'a>: HasQoSPolicy<'a> {}
