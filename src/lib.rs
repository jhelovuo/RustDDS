#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

extern crate bit_set;
extern crate bit_vec;
extern crate bytes;
extern crate num_derive;
extern crate num_traits;
extern crate speedy;
extern crate tokio_util;
extern crate uuid;

#[macro_use]
mod serialization_test;
#[macro_use]
mod checked_impl;
mod common;
mod discovery;
mod messages;
mod network;
pub(crate) mod structure;
mod test;

// Public modules
pub mod dds;
// switch off for refactoring pub mod ros2;

/// Helpers for (De)serialization and definitions of (De)serializer adapters
pub mod serialization;
