#!/bin/bash

# Generate EC-parameters
openssl ecparam -name prime256v1 -out ec_parameters.pem

# Create permissions CA
openssl req -x509 -newkey param:ec_parameters.pem -keyout permissions_ca_private_key.pem -passout file:password  -out permissions_ca_certificate.pem -days 999999 -subj "/O=Example Organization/CN=permissions_ca_common_name" 

# Sign configuration documents
openssl smime -sign -in permissive_governance_unsigned.xml -out permissive_governance.p7s -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password
openssl smime -sign -in permissive_permissions_unsigned.xml -out permissive_permissions.p7s -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password
openssl smime -sign -in test_governance_unsigned.xml -out test_governance.p7s -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password
openssl smime -sign -in test_permissions_unsigned.xml -out test_permissions.p7s -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password


# Create identity CA
openssl req -x509 -newkey param:ec_parameters.pem -keyout identity_ca_private_key.pem -passout file:password -out identity_ca_certificate.pem -days 999999 -subj "/O=Example Organization/CN=identity_ca_common_name"

# Create a certificate requests for participant identity certificates
openssl req -newkey param:ec_parameters.pem -keyout participant1_private_key.pem -nodes -out identity_certificate_requests/participant1.pem -subj "/O=Example Organization/CN=participant1_common_name"
openssl req -newkey param:ec_parameters.pem -keyout participant2_private_key.pem -nodes -out identity_certificate_requests/participant2.pem -subj "/O=Example Organization/CN=participant2_common_name"

# Sign the certificate request s
openssl x509 -req -days 999999 -in identity_certificate_requests/participant1.pem -CA identity_ca_certificate.pem -CAkey identity_ca_private_key.pem -passin file:password -out participant1_certificate.pem -set_serial 1
openssl x509 -req -days 999999 -in identity_certificate_requests/participant2.pem -CA identity_ca_certificate.pem -CAkey identity_ca_private_key.pem -passin file:password -out participant2_certificate.pem -set_serial 1

