#!/bin/bash

# Sign test configurations
openssl smime -sign -in governance_unsigned.xml -out governance.p7s -signer permissions_ca.cert.pem -inkey permissions_ca_private_key.pem -passin file:password
openssl smime -sign -in permissions_unsigned.xml -out permissions.p7s -signer permissions_ca.cert.pem -inkey permissions_ca_private_key.pem -passin file:password