use speedy::{Context, Readable, Reader, Writable, Writer};
use std::io::{Error, ErrorKind, Result};

#[derive(Debug, PartialEq)]
pub enum ParameterId {
    PID_PAD = 0x0000,
    PID_SENTINEL = 0x0001,
    PID_USER_DATA = 0x002c,
    PID_TOPIC_NAME = 0x0005,
    PID_TYPE_NAME = 0x0007,
    PID_GROUP_DATA = 0x002d,
    PID_TOPIC_DATA = 0x002e,
    PID_DURABILITY = 0x001d,
    PID_DURABILITY_SERVICE = 0x001e,
    PID_DEADLINE = 0x0023,
    PID_LATENCY_BUDGET = 0x0027,
    PID_LIVELINESS = 0x001b,
    PID_RELIABILITY = 0x001a,
    PID_LIFESPAN = 0x002b,
    PID_DESTINATION_ORDER = 0x0025,
    PID_HISTORY = 0x0040,
    PID_RESOURCE_LIMITS = 0x0041,
    PID_OWNERSHIP = 0x001f,
    PID_OWNERSHIP_STRENGTH = 0x0006,
    PID_PRESENTATION = 0x0021,
    PID_PARTITION = 0x0029,
    PID_TIME_BASED_FILTER = 0x0004,
    PID_TRANSPORT_PRIO = 0x0049,
    PID_PROTOCOL_VERSION = 0x0015,
    PID_VENDOR_ID = 0x0016,
    PID_UNICAST_LOCATOR = 0x002f,
    PID_MULTICAST_LOCATOR = 0x0030,
    PID_MULTICAST_IPADDRESS = 0x0011,
    PID_DEFAULT_UNICAST_LOCATOR = 0x0031,
    PID_DEFAULT_MULTICAST_LOCATOR = 0x0048,
    PID_METATRAFFIC_UNICAST_LOCATOR = 0x0032,
    PID_METATRAFFIC_MULTICAST_LOCATOR = 0x0033,
    PID_DEFAULT_UNICAST_IPADDRESS = 0x000c,
    PID_DEFAULT_UNICAST_PORT = 0x000e,
    PID_METATRAFFIC_UNICAST_IPADDRESS = 0x0045,
    PID_METATRAFFIC_UNICAST_PORT = 0x000d,
    PID_METATRAFFIC_MULTICAST_IPADDRESS = 0x000b,
    PID_METATRAFFIC_MULTICAST_PORT = 0x0046,
    PID_EXPECTS_INLINE_QOS = 0x0043,
    PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT = 0x0034,
    PID_PARTICIPANT_BUILTIN_ENDPOINTS = 0x0044,
    PID_PARTICIPANT_LEASE_DURATION = 0x0002,
    PID_CONTENT_FILTER_PROPERTY = 0x0035,
    PID_PARTICIPANT_GUID = 0x0050,
    PID_GROUP_GUID = 0x0052,
    PID_GROUP_ENTITYID = 0x0053,
    PID_BUILTIN_ENDPOINT_SET = 0x0058,
    PID_PROPERTY_LIST = 0x0059,
    PID_TYPE_MAX_SIZE_SERIALIZED = 0x0060,
    PID_ENTITY_NAME = 0x0062,
    PID_KEY_HASH = 0x0070,
    PID_STATUS_INFO = 0x0071,
}

impl<'a, C: Context> Readable<'a, C> for ParameterId {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self> {
        use self::ParameterId::*;
        let id = reader.read_u16()?;
        match id {
            0x0000 => Ok(PID_PAD),
            0x0001 => Ok(PID_SENTINEL),
            0x002c => Ok(PID_USER_DATA),
            0x0005 => Ok(PID_TOPIC_NAME),
            0x0007 => Ok(PID_TYPE_NAME),
            0x002d => Ok(PID_GROUP_DATA),
            0x002e => Ok(PID_TOPIC_DATA),
            0x001d => Ok(PID_DURABILITY),
            0x001e => Ok(PID_DURABILITY_SERVICE),
            0x0023 => Ok(PID_DEADLINE),
            0x0027 => Ok(PID_LATENCY_BUDGET),
            0x001b => Ok(PID_LIVELINESS),
            0x001a => Ok(PID_RELIABILITY),
            0x002b => Ok(PID_LIFESPAN),
            0x0025 => Ok(PID_DESTINATION_ORDER),
            0x0040 => Ok(PID_HISTORY),
            0x0041 => Ok(PID_RESOURCE_LIMITS),
            0x001f => Ok(PID_OWNERSHIP),
            0x0006 => Ok(PID_OWNERSHIP_STRENGTH),
            0x0021 => Ok(PID_PRESENTATION),
            0x0029 => Ok(PID_PARTITION),
            0x0004 => Ok(PID_TIME_BASED_FILTER),
            0x0049 => Ok(PID_TRANSPORT_PRIO),
            0x0015 => Ok(PID_PROTOCOL_VERSION),
            0x0016 => Ok(PID_VENDOR_ID),
            0x002f => Ok(PID_UNICAST_LOCATOR),
            0x0030 => Ok(PID_MULTICAST_LOCATOR),
            0x0011 => Ok(PID_MULTICAST_IPADDRESS),
            0x0031 => Ok(PID_DEFAULT_UNICAST_LOCATOR),
            0x0048 => Ok(PID_DEFAULT_MULTICAST_LOCATOR),
            0x0032 => Ok(PID_METATRAFFIC_UNICAST_LOCATOR),
            0x0033 => Ok(PID_METATRAFFIC_MULTICAST_LOCATOR),
            0x000c => Ok(PID_DEFAULT_UNICAST_IPADDRESS),
            0x000e => Ok(PID_DEFAULT_UNICAST_PORT),
            0x0045 => Ok(PID_METATRAFFIC_UNICAST_IPADDRESS),
            0x000d => Ok(PID_METATRAFFIC_UNICAST_PORT),
            0x000b => Ok(PID_METATRAFFIC_MULTICAST_IPADDRESS),
            0x0046 => Ok(PID_METATRAFFIC_MULTICAST_PORT),
            0x0043 => Ok(PID_EXPECTS_INLINE_QOS),
            0x0034 => Ok(PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT),
            0x0044 => Ok(PID_PARTICIPANT_BUILTIN_ENDPOINTS),
            0x0002 => Ok(PID_PARTICIPANT_LEASE_DURATION),
            0x0035 => Ok(PID_CONTENT_FILTER_PROPERTY),
            0x0050 => Ok(PID_PARTICIPANT_GUID),
            0x0052 => Ok(PID_GROUP_GUID),
            0x0053 => Ok(PID_GROUP_ENTITYID),
            0x0058 => Ok(PID_BUILTIN_ENDPOINT_SET),
            0x0059 => Ok(PID_PROPERTY_LIST),
            0x0060 => Ok(PID_TYPE_MAX_SIZE_SERIALIZED),
            0x0062 => Ok(PID_ENTITY_NAME),
            0x0070 => Ok(PID_KEY_HASH),
            0x0071 => Ok(PID_STATUS_INFO),
            _ => Err(Error::new(ErrorKind::InvalidData, "unknown ParameterId")),
        }
    }
}

impl<C: Context> Writable<C> for ParameterId {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(&'a self, writer: &mut T) -> Result<()> {
        use self::ParameterId::*;
        let id = match self {
            PID_PAD => 0x0000,
            PID_SENTINEL => 0x0001,
            PID_USER_DATA => 0x002c,
            PID_TOPIC_NAME => 0x0005,
            PID_TYPE_NAME => 0x0007,
            PID_GROUP_DATA => 0x002d,
            PID_TOPIC_DATA => 0x002e,
            PID_DURABILITY => 0x001d,
            PID_DURABILITY_SERVICE => 0x001e,
            PID_DEADLINE => 0x0023,
            PID_LATENCY_BUDGET => 0x0027,
            PID_LIVELINESS => 0x001b,
            PID_RELIABILITY => 0x001a,
            PID_LIFESPAN => 0x002b,
            PID_DESTINATION_ORDER => 0x0025,
            PID_HISTORY => 0x0040,
            PID_RESOURCE_LIMITS => 0x0041,
            PID_OWNERSHIP => 0x001f,
            PID_OWNERSHIP_STRENGTH => 0x0006,
            PID_PRESENTATION => 0x0021,
            PID_PARTITION => 0x0029,
            PID_TIME_BASED_FILTER => 0x0004,
            PID_TRANSPORT_PRIO => 0x0049,
            PID_PROTOCOL_VERSION => 0x0015,
            PID_VENDOR_ID => 0x0016,
            PID_UNICAST_LOCATOR => 0x002f,
            PID_MULTICAST_LOCATOR => 0x0030,
            PID_MULTICAST_IPADDRESS => 0x0011,
            PID_DEFAULT_UNICAST_LOCATOR => 0x0031,
            PID_DEFAULT_MULTICAST_LOCATOR => 0x0048,
            PID_METATRAFFIC_UNICAST_LOCATOR => 0x0032,
            PID_METATRAFFIC_MULTICAST_LOCATOR => 0x0033,
            PID_DEFAULT_UNICAST_IPADDRESS => 0x000c,
            PID_DEFAULT_UNICAST_PORT => 0x000e,
            PID_METATRAFFIC_UNICAST_IPADDRESS => 0x0045,
            PID_METATRAFFIC_UNICAST_PORT => 0x000d,
            PID_METATRAFFIC_MULTICAST_IPADDRESS => 0x000b,
            PID_METATRAFFIC_MULTICAST_PORT => 0x0046,
            PID_EXPECTS_INLINE_QOS => 0x0043,
            PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT => 0x0034,
            PID_PARTICIPANT_BUILTIN_ENDPOINTS => 0x0044,
            PID_PARTICIPANT_LEASE_DURATION => 0x0002,
            PID_CONTENT_FILTER_PROPERTY => 0x0035,
            PID_PARTICIPANT_GUID => 0x0050,
            PID_GROUP_GUID => 0x0052,
            PID_GROUP_ENTITYID => 0x0053,
            PID_BUILTIN_ENDPOINT_SET => 0x0058,
            PID_PROPERTY_LIST => 0x0059,
            PID_TYPE_MAX_SIZE_SERIALIZED => 0x0060,
            PID_ENTITY_NAME => 0x0062,
            PID_KEY_HASH => 0x0070,
            PID_STATUS_INFO => 0x0071,
        };
        writer.write_u16(id)
    }
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
