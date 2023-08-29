These are example configuration files to be used with tests and examples.

The certificates have been generated using OpenSSL according to [OpenSSL Cookbook](https://www.feistyduck.com/library/openssl-cookbook/online/).

PEM pass phrase: `password123`

Create `permissions_ca.pem` and `permissions_ca_private_key.pem` with RSA-PSS (for some reason verification still fails):\
`openssl req -x509 -newkey rsa-pss -keyout permissions_ca_private_key.pem -out permissions_ca.pem -days 999999 -subj "/O=Example Organization/CN=permissions_ca_common_name"`\
_

OR

Create `permissions_ca.pem` and `permissions_ca_private_key.pem` with elliptic curves:\
`openssl ecparam -name prime256v1 -out ec_parameters.pem`\
_\
`openssl req -x509 -newkey param:ec_parameters.pem -keyout permissions_ca_private_key.pem -out permissions_ca.pem -days 999999 -subj "/O=Example Organization/CN=permissions_ca_common_name"`\
_


Inspect the certificate:\
`openssl x509 -text -in permissions_ca.pem -noout`\
_

Sign configuration documents:\
`openssl smime -sign -in permissive_governance_unsigned.xml -out permissive_governance.p7s -signer permissions_ca.pem -inkey permissions_ca_private_key.pem`\
_\
`openssl smime -sign -in permissive_permissions_unsigned.xml -out permissive_permissions.p7s -signer permissions_ca.pem -inkey permissions_ca_private_key.pem`\
_