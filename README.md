# AtosDDS

The Data Distribution Service for real-time systems (DDS) is an Object Management Group (OMG) machine-to-machine connectivity framework that aims to enable scalable, real-time, dependable, high-performance and interoperable data exchanges using a publish–subscribe pattern. DDS addresses the needs of applications like air-traffic control, smart grid management, autonomous vehicles, robotics, transportation systems, power generation, medical devices, simulation and testing, aerospace and defense, and other applications that require real-time data exchange [[Wiki]][wiki-dds-url].

This is a pure Rust implementation of DDS. We have tried to translate the key ideas of the DDS application interface to Rust concepts, but also follow Rust conventions, so the API is not quite the same as with DDS specification.

# Intentional deviations from DDS specification

## Rationale

The [DDS][omg-dds-spec-url] 1.4 specification specifies an object model and a set of APIs for those objects that constitude the DDS specification. The design of these APIs in e.g. naming conventions and memory management semantics, does not quite fit the Rust world. We have tried to design where important DDS ideas are preserved and implemented, but in a manner suitable to Rust. These design compromises should be apprent only on the application-facing API of DDS. The network side is still aiming to be fully interoperable with existing DDS implementations.

## Class Hierarchy

The DDS specifies a class hierarchy, which is part of the API. That hierarchy is not necessarily followed, because Rust does not use inheritance and derived classes in the same sense as e.g. C++.

## Naming Conventions

We have tried to follow Rust naming conventions.

## Data listeners and WaitSets

DDS provides two alternative methods for waiting arriving data, namely WaitSets and Listeners. We have chosen to replace these by using the non-blocking IO API from [mio][metal-io-url] crate. The DDS DataReader objects can be used with the mio `Poll` interface. It should be possible to implement oter APIs, such as an async API on top of that.

## Instance Handles

DDS uses "instance handles", which behave like pointers to objects managed by the DDS implementation. This does not seem to mix well with Rust memory handling, so we have chosen to not implement those.

An instance handle can be used to refer to refer to data values (samples) with a specific key. We have written the API to use directly the key insted, as that seems semantically equivalent.

## Return codes

The list of standard method return codes specifed by DDS (section 2.2.1.1) is modified, in particaular:

* The `OK` code is not used to indicate successful operation. Success or failure is indicated using the `Result` type.
* The `TIMEOUT` code is not used. Timeouts should be indicated as `Result::Err` or `Option::None`.
* The generic `ERROR` code should not be used, but a more specific value instead.
* `NO_DATA` is not used. The absence of data should be encoded as `Option::None`.

## DataReader and DataWriter interfaces

The DDS specification specifies multiple functions to read received data samples out of a DataReader:

* `read`: Borrows desrialized data objects from DataReader. Marks them read. Can be read again, if already read samples are requested.
* `take`: Like read, but removes returned objects from DataReader, so they cannot be read again.
* `read_w_condition`, `take_w_condition`: Read/take samples that match specified condition.
* `read_next_sample`, `take_next_sample`: Read/take next non-previously accessed sample.
* `read_instance`, `take_instance`: Read/take samples belonging to a single instance (having the same key).
* `read_next_instance`, `take_next_instance`: Combination of `_next` and `_instance`.
* `read_next_instance_w_condition`, `take_next_instance_w_condition`: Combination of `_next` , `_instance` , and `_w_condition`.

We have decided to not implement all 12 of these. Instead, we implement smaller collection of methods:

* `read` : Always specify a read condition, but it is easy to specify a null condition, that reads unconditionally.
* `take` : As above.
* `read_instance`, `take_instance`: Access samples belonging to a single key. Instance is specified by a key, not InstanceHandle. Accepts a read condition.

There are also methods  `read_next_sample`, `take_next_sample` , but these are essentially simplification wrappers for read/take.



# Current status

This is still work-in-progress. The most immediate goal is to have enough functionality to be able to communicate with [ROS2][ros2-url] software.

# Based on rtps-rs

The RTPS implementation used here is derived from [rtps-rs][klapeyron-rtps-rs-url].

[wiki-dds-url]: https://en.wikipedia.org/wiki/Data_Distribution_Service
[omg-rtps-url]: https://www.omg.org/spec/DDSI-RTPS/2.3
[omg-dds-spec-url]: https://www.omg.org/spec/DDS/About-DDS/
[klapeyron-rtps-rs-url]: https://github.com/Klapeyron/rtps-rs
[ros2-url]: https://index.ros.org/doc/ros2/
[metal-io-url]: https://docs.rs/mio/0.6.22/mio/