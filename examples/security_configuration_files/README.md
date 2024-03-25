These are example configuration files to be used with tests and examples.

The certificates have been generated using OpenSSL according to [OpenSSL Cookbook](https://www.feistyduck.com/library/openssl-cookbook/online/) and [Signing certificates](https://www.ibm.com/docs/en/license-metric-tool?topic=certificate-step-2-signing-certificates).

PEM pass phrase in the file `password`: `password123` 

Create Permissions CA files `permissions_ca.cert.pem` and `permissions_ca_private_key.pem` with elliptic curves:\
`openssl ecparam -name prime256v1 -out ec_parameters.pem`\
_\
`openssl req -x509 -newkey param:ec_parameters.pem -keyout permissions_ca_private_key.pem -passout file:password  -out permissions_ca.cert.pem -days 999999 -subj "/O=Example Organization/CN=permissions_ca_common_name"`\


Inspect the certificate:\
`openssl x509 -text -in permissions_ca.cert.pem -noout`\
_

Sign configuration documents:\
`openssl smime -sign -in governance_unsigned.xml -text -out governance.p7s -signer permissions_ca.cert.pem -inkey permissions_ca_private_key.pem -passin file:password`\
_\
`openssl smime -sign -in permissions_unsigned.xml -text -out permissions.p7s -signer permissions_ca.cert.pem -inkey permissions_ca_private_key.pem -passin file:password`\
_


Create Identity CA files `identity_ca.cert.pem` and `identity_ca_private_key.pem`:\
`openssl req -x509 -newkey param:ec_parameters.pem -keyout identity_ca_private_key.pem -passout file:password -out identity_ca.cert.pem -days 999999 -subj "/O=Example Organization/CN=identity_ca_common_name"`\
_


Create a certificate request and make the Identity CA sign it. This creates the participant's private key `key.pem` and the identity certificate `cert.pem`. WARNING: password-encrypted private keys are not yet supported for identity certificates, so we use the `-nodes` option for the example, which is not advised:\
`openssl req -newkey param:ec_parameters.pem -keyout key.pem -nodes -out identity_certificate_request.pem -subj "/O=Example Organization/CN=participant1_common_name"`\
_\
`openssl x509 -req -days 999999 -in identity_certificate_request.pem -CA identity_ca.cert.pem -CAkey identity_ca_private_key.pem -passin file:password -out cert.pem -set_serial 1`\
_



# Using Hardware Security Module (PKCS#11 / Cryptoki)

## Provisioning Method 1: Generate keys using OpenSSL on CPU as usual

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

We need a 256-bit Elliptic Curve Key for the prime256v1 curve, as generated above, in `key.pem`.

`softhsm2-util --import key.pem --token example_token --pin 1234 --label test_private_key --id f00d`

Use the `pkcs11-dump` utility to check what we imported:

```$ pkcs11-dump dump  /usr/lib/softhsm/libsofthsm2.so 2046880677 1234```


## Provisioning Method 2: Generate keys in HSM

The advantage of this method is that the private key never leaves the HSM.

### Ask HSM to generate a key pair

`$ pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so --token-label ec_key --pin 1234 --keypairgen --key-type EC:prime256v1 --label id_key --id d00f`

### Extract the public key to a Certificate Signing Request.

TODO (openssl)

### Sign the CSR using Identity CA's cert and private key

TODO (openssl)


