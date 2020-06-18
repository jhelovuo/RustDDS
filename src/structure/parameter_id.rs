use speedy::{Readable, Writable};

#[derive(Debug, PartialEq, Readable, Writable)]
pub struct ParameterId {
  value: u16,
}

impl ParameterId {
  pub const PID_PAD: ParameterId = ParameterId { value: 0x0000 };
  pub const PID_SENTINEL: ParameterId = ParameterId { value: 0x0001 };
  pub const PID_USER_DATA: ParameterId = ParameterId { value: 0x002c };
  pub const PID_TOPIC_NAME: ParameterId = ParameterId { value: 0x0005 };
  pub const PID_TYPE_NAME: ParameterId = ParameterId { value: 0x0007 };
  pub const PID_GROUP_DATA: ParameterId = ParameterId { value: 0x002d };
  pub const PID_TOPIC_DATA: ParameterId = ParameterId { value: 0x002e };
  pub const PID_DURABILITY: ParameterId = ParameterId { value: 0x001d };
  pub const PID_DURABILITY_SERVICE: ParameterId = ParameterId { value: 0x001e };
  pub const PID_DEADLINE: ParameterId = ParameterId { value: 0x0023 };
  pub const PID_LATENCY_BUDGET: ParameterId = ParameterId { value: 0x0027 };
  pub const PID_LIVELINESS: ParameterId = ParameterId { value: 0x001b };
  pub const PID_RELIABILITY: ParameterId = ParameterId { value: 0x001a };
  pub const PID_LIFESPAN: ParameterId = ParameterId { value: 0x002b };
  pub const PID_DESTINATION_ORDER: ParameterId = ParameterId { value: 0x0025 };
  pub const PID_HISTORY: ParameterId = ParameterId { value: 0x0040 };
  pub const PID_RESOURCE_LIMITS: ParameterId = ParameterId { value: 0x0041 };
  pub const PID_OWNERSHIP: ParameterId = ParameterId { value: 0x001f };
  pub const PID_OWNERSHIP_STRENGTH: ParameterId = ParameterId { value: 0x0006 };
  pub const PID_PRESENTATION: ParameterId = ParameterId { value: 0x0021 };
  pub const PID_PARTITION: ParameterId = ParameterId { value: 0x0029 };
  pub const PID_TIME_BASED_FILTER: ParameterId = ParameterId { value: 0x0004 };
  pub const PID_TRANSPORT_PRIO: ParameterId = ParameterId { value: 0x0049 };
  pub const PID_PROTOCOL_VERSION: ParameterId = ParameterId { value: 0x0015 };
  pub const PID_VENDOR_ID: ParameterId = ParameterId { value: 0x0016 };
  pub const PID_UNICAST_LOCATOR: ParameterId = ParameterId { value: 0x002f };
  pub const PID_MULTICAST_LOCATOR: ParameterId = ParameterId { value: 0x0030 };
  pub const PID_MULTICAST_IPADDRESS: ParameterId = ParameterId { value: 0x0011 };
  pub const PID_DEFAULT_UNICAST_LOCATOR: ParameterId = ParameterId { value: 0x0031 };
  pub const PID_DEFAULT_MULTICAST_LOCATOR: ParameterId = ParameterId { value: 0x0048 };
  pub const PID_METATRAFFIC_UNICAST_LOCATOR: ParameterId = ParameterId { value: 0x0032 };
  pub const PID_METATRAFFIC_MULTICAST_LOCATOR: ParameterId = ParameterId { value: 0x0033 };
  pub const PID_DEFAULT_UNICAST_IPADDRESS: ParameterId = ParameterId { value: 0x000c };
  pub const PID_DEFAULT_UNICAST_PORT: ParameterId = ParameterId { value: 0x000e };
  pub const PID_METATRAFFIC_UNICAST_IPADDRESS: ParameterId = ParameterId { value: 0x0045 };
  pub const PID_METATRAFFIC_UNICAST_PORT: ParameterId = ParameterId { value: 0x000d };
  pub const PID_METATRAFFIC_MULTICAST_IPADDRESS: ParameterId = ParameterId { value: 0x000b };
  pub const PID_METATRAFFIC_MULTICAST_PORT: ParameterId = ParameterId { value: 0x0046 };
  pub const PID_EXPECTS_INLINE_QOS: ParameterId = ParameterId { value: 0x0043 };
  pub const PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT: ParameterId = ParameterId { value: 0x0034 };
  pub const PID_PARTICIPANT_BUILTIN_ENDPOINTS: ParameterId = ParameterId { value: 0x0044 };
  pub const PID_PARTICIPANT_LEASE_DURATION: ParameterId = ParameterId { value: 0x0002 };
  pub const PID_CONTENT_FILTER_PROPERTY: ParameterId = ParameterId { value: 0x0035 };
  pub const PID_PARTICIPANT_GUID: ParameterId = ParameterId { value: 0x0050 };
  pub const PID_GROUP_GUID: ParameterId = ParameterId { value: 0x0052 };
  pub const PID_GROUP_ENTITYID: ParameterId = ParameterId { value: 0x0053 };
  pub const PID_BUILTIN_ENDPOINT_SET: ParameterId = ParameterId { value: 0x0058 };
  pub const PID_PROPERTY_LIST: ParameterId = ParameterId { value: 0x0059 };
  pub const PID_TYPE_MAX_SIZE_SERIALIZED: ParameterId = ParameterId { value: 0x0060 };
  pub const PID_ENTITY_NAME: ParameterId = ParameterId { value: 0x0062 };
  pub const PID_KEY_HASH: ParameterId = ParameterId { value: 0x0070 };
  pub const PID_STATUS_INFO: ParameterId = ParameterId { value: 0x0071 };
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = ParameterId,
  {
      pid_pad,
      ParameterId::PID_PAD,
      le = [0x00, 0x00],
      be = [0x00, 0x00]
  },
  {
      pid_sentinel,
      ParameterId::PID_SENTINEL,
      le = [0x01, 0x00],
      be = [0x00, 0x01]
  },
  {
      pid_user_data,
      ParameterId::PID_USER_DATA,
      le = [0x2c, 0x00],
      be = [0x00, 0x2c]
  },
  {
      pid_topic_name,
      ParameterId::PID_TOPIC_NAME,
      le = [0x05, 0x00],
      be = [0x00, 0x05]
  },
  {
      pid_type_name,
      ParameterId::PID_TYPE_NAME,
      le = [0x07, 0x00],
      be = [0x00, 0x07]
  },
  {
      pid_group_data,
      ParameterId::PID_GROUP_DATA,
      le = [0x2d, 0x00],
      be = [0x00, 0x2d]
  },
  {
      pid_topic_data,
      ParameterId::PID_TOPIC_DATA,
      le = [0x2e, 0x00],
      be = [0x00, 0x2e]
  },
  {
      pid_durability,
      ParameterId::PID_DURABILITY,
      le = [0x1d, 0x00],
      be = [0x00, 0x1d]
  },
  {
      pid_durability_service,
      ParameterId::PID_DURABILITY_SERVICE,
      le = [0x1e, 0x00],
      be = [0x00, 0x1e]
  },
  {
      pid_deadline,
      ParameterId::PID_DEADLINE,
      le = [0x23, 0x00],
      be = [0x00, 0x23]
  },
  {
      pid_latency_budget,
      ParameterId::PID_LATENCY_BUDGET,
      le = [0x27, 0x00],
      be = [0x00, 0x27]
  },
  {
      pid_liveliness,
      ParameterId::PID_LIVELINESS,
      le = [0x1b, 0x00],
      be = [0x00, 0x1b]
  },
  {
      pid_reliability,
      ParameterId::PID_RELIABILITY,
      le = [0x1a, 0x00],
      be = [0x00, 0x1a]
  },
  {
      pid_lifespan,
      ParameterId::PID_LIFESPAN,
      le = [0x2b, 0x00],
      be = [0x00, 0x2b]
  },
  {
      pid_destination_order,
      ParameterId::PID_DESTINATION_ORDER,
      le = [0x25, 0x00],
      be = [0x00, 0x25]
  },
  {
      pid_history,
      ParameterId::PID_HISTORY,
      le = [0x40, 0x00],
      be = [0x00, 0x40]
  },
  {
      pid_resource_limits,
      ParameterId::PID_RESOURCE_LIMITS,
      le = [0x41, 0x00],
      be = [0x00, 0x41]
  },
  {
      pid_ownership,
      ParameterId::PID_OWNERSHIP,
      le = [0x1f, 0x00],
      be = [0x00, 0x1f]
  },
  {
      pid_ownership_strength,
      ParameterId::PID_OWNERSHIP_STRENGTH,
      le = [0x06, 0x00],
      be = [0x00, 0x06]
  },
  {
      pid_presentation,
      ParameterId::PID_PRESENTATION,
      le = [0x21, 0x00],
      be = [0x00, 0x21]
  },
  {
      pid_partition,
      ParameterId::PID_PARTITION,
      le = [0x29, 0x00],
      be = [0x00, 0x29]
  },
  {
      pid_time_based_filter,
      ParameterId::PID_TIME_BASED_FILTER,
      le = [0x04, 0x00],
      be = [0x00, 0x04]
  },
  {
      pid_transport_prio,
      ParameterId::PID_TRANSPORT_PRIO,
      le = [0x49, 0x00],
      be = [0x00, 0x49]
  },
  {
      pid_protocol_version,
      ParameterId::PID_PROTOCOL_VERSION,
      le = [0x15, 0x00],
      be = [0x00, 0x15]
  },
  {
      pid_vendor_id,
      ParameterId::PID_VENDOR_ID,
      le = [0x16, 0x00],
      be = [0x00, 0x16]
  },
  {
      pid_unicast_locator,
      ParameterId::PID_UNICAST_LOCATOR,
      le = [0x2f, 0x00],
      be = [0x00, 0x2f]
  },
  {
      pid_multicast_locator,
      ParameterId::PID_MULTICAST_LOCATOR,
      le = [0x30, 0x00],
      be = [0x00, 0x30]
  },
  {
      pid_multicast_ipaddress,
      ParameterId::PID_MULTICAST_IPADDRESS,
      le = [0x11, 0x00],
      be = [0x00, 0x11]
  },
  {
      pid_default_unicast_locator,
      ParameterId::PID_DEFAULT_UNICAST_LOCATOR,
      le = [0x31, 0x00],
      be = [0x00, 0x31]
  },
  {
      pid_default_multicast_locator,
      ParameterId::PID_DEFAULT_MULTICAST_LOCATOR,
      le = [0x48, 0x00],
      be = [0x00, 0x48]
  },
  {
      pid_metatraffic_unicast_locator,
      ParameterId::PID_METATRAFFIC_UNICAST_LOCATOR,
      le = [0x32, 0x00],
      be = [0x00, 0x32]
  },
  {
      pid_metatraffic_multicast_locator,
      ParameterId::PID_METATRAFFIC_MULTICAST_LOCATOR,
      le = [0x33, 0x00],
      be = [0x00, 0x33]
  },
  {
      pid_default_unicast_ipaddress,
      ParameterId::PID_DEFAULT_UNICAST_IPADDRESS,
      le = [0x0c, 0x00],
      be = [0x00, 0x0c]
  },
  {
      pid_default_unicast_port,
      ParameterId::PID_DEFAULT_UNICAST_PORT,
      le = [0x0e, 0x00],
      be = [0x00, 0x0e]
  },
  {
      pid_metatraffic_unicast_ipaddress,
      ParameterId::PID_METATRAFFIC_UNICAST_IPADDRESS,
      le = [0x45, 0x00],
      be = [0x00, 0x45]
  },
  {
      pid_metatraffic_unicast_port,
      ParameterId::PID_METATRAFFIC_UNICAST_PORT,
      le = [0x0d, 0x00],
      be = [0x00, 0x0d]
  },
  {
      pid_metatraffic_multicast_ipaddress,
      ParameterId::PID_METATRAFFIC_MULTICAST_IPADDRESS,
      le = [0x0b, 0x00],
      be = [0x00, 0x0b]
  },
  {
      pid_metatraffic_multicast_port,
      ParameterId::PID_METATRAFFIC_MULTICAST_PORT,
      le = [0x46, 0x00],
      be = [0x00, 0x46]
  },
  {
      pid_expects_inline_qos,
      ParameterId::PID_EXPECTS_INLINE_QOS,
      le = [0x43, 0x00],
      be = [0x00, 0x43]
  },
  {
      pid_participant_manual_liveliness_count,
      ParameterId::PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT,
      le = [0x34, 0x00],
      be = [0x00, 0x34]
  },
  {
      pid_participant_builtin_endpoints,
      ParameterId::PID_PARTICIPANT_BUILTIN_ENDPOINTS,
      le = [0x44, 0x00],
      be = [0x00, 0x44]
  },
  {
      pid_participant_lease_duration,
      ParameterId::PID_PARTICIPANT_LEASE_DURATION,
      le = [0x02, 0x00],
      be = [0x00, 0x02]
  },
  {
      pid_content_filter_property,
      ParameterId::PID_CONTENT_FILTER_PROPERTY,
      le = [0x35, 0x00],
      be = [0x00, 0x35]
  },
  {
      pid_participant_guid,
      ParameterId::PID_PARTICIPANT_GUID,
      le = [0x50, 0x00],
      be = [0x00, 0x50]
  },
  {
      pid_group_guid,
      ParameterId::PID_GROUP_GUID,
      le = [0x52, 0x00],
      be = [0x00, 0x52]
  },
  {
      pid_group_entityid,
      ParameterId::PID_GROUP_ENTITYID,
      le = [0x53, 0x00],
      be = [0x00, 0x53]
  },
  {
      pid_builtin_endpoint_set,
      ParameterId::PID_BUILTIN_ENDPOINT_SET,
      le = [0x58, 0x00],
      be = [0x00, 0x58]
  },
  {
      pid_property_list,
      ParameterId::PID_PROPERTY_LIST,
      le = [0x59, 0x00],
      be = [0x00, 0x59]
  },
  {
      pid_type_max_size_serialized,
      ParameterId::PID_TYPE_MAX_SIZE_SERIALIZED,
      le = [0x60, 0x00],
      be = [0x00, 0x60]
  },
  {
      pid_entity_name,
      ParameterId::PID_ENTITY_NAME,
      le = [0x62, 0x00],
      be = [0x00, 0x62]
  },
  {
      pid_key_hash,
      ParameterId::PID_KEY_HASH,
      le = [0x70, 0x00],
      be = [0x00, 0x70]
  },
  {
      pid_status_info,
      ParameterId::PID_STATUS_INFO,
      le = [0x71, 0x00],
      be = [0x00, 0x71]
  });
}
