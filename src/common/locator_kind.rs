use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Visitor};
use std::{fmt};

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
#[repr(i32)]
pub enum LocatorKind_t {
    LOCATOR_KIND_INVALID = -1,
    LOCATOR_KIND_RESERVED = 0,
    LOCATOR_KIND_UDPv4 = 1,
    LOCATOR_KIND_UDPv6 = 2
}

impl Serialize for LocatorKind_t {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(*self as i32)
    }
}

impl<'de> Deserialize<'de> for LocatorKind_t {
    fn deserialize<D>(deserializer: D) -> Result<LocatorKind_t, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct I32Visitor;

        impl<'de> Visitor<'de> for I32Visitor {
            type Value = LocatorKind_t;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an integer between -1 and 2")
            }

            fn visit_i32<E>(self, value: i32) -> Result<LocatorKind_t, E>
            where
                E: Error,
            {
                match value {
                    -1 => Ok(LocatorKind_t::LOCATOR_KIND_INVALID),
                    0 => Ok(LocatorKind_t::LOCATOR_KIND_RESERVED),
                    1 => Ok(LocatorKind_t::LOCATOR_KIND_UDPv4),
                    2 => Ok(LocatorKind_t::LOCATOR_KIND_UDPv6),
                    _ => Err(E::custom(format!("LocatorKind_t out of range: {}", value)))
                }
            }
        }

        deserializer.deserialize_i32(I32Visitor)
    }
}
