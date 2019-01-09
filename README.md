[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][licence-badge]][licence-url]
[![Travis Build Status][travis-badge]][travis-url]

# The Real-time Publish-Subscribe Protocol for Rust

This is the implementation of the Real-time Publish-Subscribe Protocol (RTPS) DDS Interoperability Wire Protocol for Rust.

The Data Distribution Service for real-time systems (DDS) is an Object Management Group (OMG) machine-to-machine connectivity framework that aims to enable scalable, real-time, dependable, high-performance and interoperable data exchanges using a publish–subscribe pattern. DDS addresses the needs of applications like air-traffic control, smart grid management, autonomous vehicles, robotics, transportation systems, power generation, medical devices, simulation and testing, aerospace and defense, and other applications that require real-time data exchange [[Wiki]][wiki-dds-url].

### Intended Features
The objectives of this implementation are (incomplete list):
* Implementing RTPS according to specification [[RTPS-2.2]][omg-rtps-url]
* Integrating into the Rust-Tokio event system for async IO.
* Interoperability with OpenDDS, and other RTPS/DDS implementations.

[crates-badge]: https://img.shields.io/crates/v/rtps-rs.svg
[crates-url]: https://crates.io/crates/rtps-rs
[licence-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[licence-url]: LICENSE.md
[travis-badge]: https://travis-ci.com/Klapeyron/rtps-rs.svg?branch=master
[travis-url]: https://travis-ci.com/Klapeyron/rtps-rs
[wiki-dds-url]: https://en.wikipedia.org/wiki/Data_Distribution_Service
[omg-rtps-url]:https://www.omg.org/spec/DDSI-RTPS/About-DDSI-RTPS/ 
