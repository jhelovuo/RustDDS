use crate::dds::qos::HasQoSPolicy;

/// Intended to somewhat describe DDS 2.2.2.1.1 Entity Class
pub trait DDSEntity: HasQoSPolicy {}
