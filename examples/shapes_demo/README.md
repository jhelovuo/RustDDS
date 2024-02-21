# Shapes Demo Example

This shapes demo example is meant to verify and demonstrate compatibility with other DDS implementations.

1. Get some others DDS implementations shapes demo. Eg. https://www.eprosima.com/index.php/products-all/eprosima-shapes-demo and start it to, for example, publish into the `Square` topic.
2. Change to examples/shapes_demo subdirectory to find a logging configuration file. Otherwise, the demo runs with default logging.
3. Run the RustDDS shapes demo with appropriate options, e.g.,

        cargo run --example=shapes_demo -- -S -t Square

    to subscribe to the Square topic. To run the same with security enabled, use

        cargo run --features=security --example=shapes_demo -- -S -t Square --security=../security_configuration_files

    where the `security` argument points in this case to the `examples/security_configuration_files` directory.

4. To exit shapes demo press  'Ctrl + C' 

