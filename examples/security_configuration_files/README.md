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
`openssl smime -sign -in governance_unsigned.xml -out governance.p7s -signer permissions_ca.cert.pem -inkey permissions_ca_private_key.pem -passin file:password`\
_\
`openssl smime -sign -in permissions_unsigned.xml -out permissions.p7s -signer permissions_ca.cert.pem -inkey permissions_ca_private_key.pem -passin file:password`\
_


Create Identity CA files `identity_ca.cert.pem` and `identity_ca_private_key.pem`:\
`openssl req -x509 -newkey param:ec_parameters.pem -keyout identity_ca_private_key.pem -passout file:password -out identity_ca.cert.pem -days 999999 -subj "/O=Example Organization/CN=identity_ca_common_name"`\
_


Create a certificate request and make the Identity CA sign it. This creates the participant's private key `key.pem` and the identity certificate `cert.pem`. WARNING: password-encrypted private keys are not yet supported for identity certificates, so we use the `-nodes` option for the example, which is not advised:\
`openssl req -newkey param:ec_parameters.pem -keyout key.pem -nodes -out identity_certificate_request.pem -subj "/O=Example Organization/CN=participant1_common_name"`\
_\
`openssl x509 -req -days 999999 -in identity_certificate_request.pem -CA identity_ca.cert.pem -CAkey identity_ca_private_key.pem -passin file:password -out cert.pem -set_serial 1`\
_
