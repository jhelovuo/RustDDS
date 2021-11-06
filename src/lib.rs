//! A pure Rust implementation of Data Distribution Service (DDS).
//!
//! DDS is an object-oriented API [specified](https://www.omg.org/spec/DDSI-RTPS/2.3) by
//! the Object Management Group.
//!
//! DDS communicates over the network using the [RTPS](https://www.omg.org/spec/DDSI-RTPS/2.3)
//! protocol, which by default runs over UDP/IP.
//!
//! This implementation does not attempt to make an accurate implementation of
//! the DDS object API, as it would be quite unnatural to use in Rust as such.
//! However, we aim for functional compatibility, while at the same time using
//! Rust techniques and conventions.
//!
//! Additionally, there is a [ROS2](https://index.ros.org/doc/ros2/) interface, that is simpler to use than DDS
//! when communicating to ROS2 components.

#![warn(clippy::all)]
#![allow(
  clippy::option_map_unit_fn,
  // option_map_unit_fn suggests changing option.map( ) with () return value to if let -construct,
  // but that may break code flow.
  dead_code
)]

#[macro_use]
mod serialization_test;
#[macro_use]
mod checked_impl;
mod discovery;
mod messages;
mod network;
pub(crate) mod structure;

#[cfg(test)]
mod test;

// Public modules
pub mod dds;
pub mod ros2;

/// Helpers for (De)serialization and definitions of (De)serializer adapters
pub mod serialization;
