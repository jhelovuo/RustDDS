# AtosDDS

The Data Distribution Service for real-time systems (DDS) is an Object Management Group (OMG) machine-to-machine connectivity framework that aims to enable scalable, real-time, dependable, high-performance and interoperable data exchanges using a publish–subscribe pattern. DDS addresses the needs of applications like air-traffic control, smart grid management, autonomous vehicles, robotics, transportation systems, power generation, medical devices, simulation and testing, aerospace and defense, and other applications that require real-time data exchange [[Wiki]][wiki-dds-url].

This is a pure Rust implementation of DDS. We have tried to translate the key ideas of the DDS application interface to Rust concepts, but also follow Rust conventions, so the API is not quite the same as with DDS specification.

# Current status

This is still work-in-progress. The most immediate goal is to have enough functionality to be able to communicate with [ROS2][ros2-url] software.

# Based on rtps-rs

The RTPS implementation used here is derived from [rtps-rs][klapeyron-rtps-rs-url].

[wiki-dds-url]: https://en.wikipedia.org/wiki/Data_Distribution_Service
[omg-rtps-url]: https://www.omg.org/spec/DDSI-RTPS/2.3
[klapeyron-rtps-rs-url]: https://github.com/Klapeyron/rtps-rs
[ros2-url]: https://index.ros.org/doc/ros2/