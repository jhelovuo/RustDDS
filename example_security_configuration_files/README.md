These are example configuration files to be used with tests and examples.

The certificates have been generated using OpenSSL according to [OpenSSL Cookbook](https://www.feistyduck.com/library/openssl-cookbook/online/) and [Signing certificates](https://www.ibm.com/docs/en/license-metric-tool?topic=certificate-step-2-signing-certificates).

PEM pass phrase in the file `password`: `password123` 

Create `permissions_ca_certificate.pem` and `permissions_ca_private_key.pem` with elliptic curves:\
`openssl ecparam -name prime256v1 -out ec_parameters.pem`\
_\
`openssl req -x509 -newkey param:ec_parameters.pem -keyout permissions_ca_private_key.pem -passout file:password  -out permissions_ca_certificate.pem -days 999999 -subj "/O=Example Organization/CN=permissions_ca_common_name"`\
_

OR

Create `permissions_ca_certificate.pem` and `permissions_ca_private_key.pem` with RSA-PSS (for some reason verification still fails):\
`openssl req -x509 -newkey rsa-pss -keyout permissions_ca_private_key.pem -passout file:password -out permissions_ca_certificate.pem -days 999999 -subj "/O=Example Organization/CN=permissions_ca_common_name"`\
_





Inspect the certificate:\
`openssl x509 -text -in permissions_ca_certificate.pem -noout`\
_

Sign configuration documents:\
`openssl smime -sign -in permissive_governance_unsigned.xml -out permissive_governance.p7s -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password`\
_\
`openssl smime -sign -in permissive_permissions_unsigned.xml -out permissive_permissions.p7s -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password`\
_


Create `identity_ca.pem`:\
`openssl req -x509 -newkey param:ec_parameters.pem -keyout identity_ca_private_key.pem -passout file:password -out identity_ca_certificate.pem -days 999999 -subj "/O=Example Organization/CN=identity_ca_common_name"`\
_


Create a certificate request and sign it to create `participant1_certificate.pem`. WARNING: password-encrypted private keys are not yet supported for identity certificates, so we use the `-nodes` option for the example, which is not advised:\
`openssl req -newkey param:ec_parameters.pem -keyout participant1_private_key.pem -nodes -out identity_certificate_requests/participant1.pem -subj "/O=Example Organization/CN=participant1_common_name"`\
_\
`openssl x509 -req -days 999999 -in identity_certificate_requests/participant1.pem -CA identity_ca_certificate.pem -CAkey identity_ca_private_key.pem -passin file:password -out participant1_certificate.pem -set_serial 1`\
_
