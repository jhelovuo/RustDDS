#!/bin/bash

# Sign test configurations
openssl smime -sign -in test_governance_unsigned.xml -out test_governance.p7s -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password
openssl smime -sign -in test_permissions_unsigned.xml -out test_permissions.p7s -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password

# For some reason FastDDS expects a content type prefix within the file
#cat FastDDS_xml_prefix test_governance_unsigned.xml > FastDDS_governance_unsigned.xml
#openssl smime -sign -in FastDDS_governance_unsigned.xml -out FastDDS_governance.smime -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password
#cat FastDDS_xml_prefix test_permissions_unsigned.xml > FastDDS_permissions_unsigned.xml
#openssl smime -sign -in FastDDS_permissions_unsigned.xml -out FastDDS_permissions.smime -signer permissions_ca_certificate.pem -inkey permissions_ca_private_key.pem -passin file:password