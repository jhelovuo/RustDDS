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


# Using Hardware Security Module (PKCS#11 / Cryptoki)

Initialize an emulated HSM. We call it `example_token`

`$ softhsm2-util --init-token --free --label example_token --pin 1234 --so-pin 12345`


`$ softhsm2-util --show-slots`

```
Slot 2046880677
    Slot info:
        Description:      SoftHSM slot ID 0x7a00eba5                            
        Manufacturer ID:  SoftHSM project
        Hardware version: 2.6
        Firmware version: 2.6
        Token present:    yes
    Token info:
        Manufacturer ID:  SoftHSM project
        Model:            SoftHSM v2
        Hardware version: 2.6
        Firmware version: 2.6
        Serial number:    da58e2f47a00eba5
        Initialized:      yes
        User PIN init.:   yes
        Label:            example_token

```

`softhsm2-util --import key.pem --token example_token --pin 1234 --label test_private_key --id f00d`

Use the `pkcs11-dump` utility to check what we imported:

```$ pkcs11-dump dump  /usr/lib/softhsm/libsofthsm2.so 2046880677 1234```

