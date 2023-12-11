# Security in RustDDS

Security support in DDS means the abilities to

* Cryptographically authenticate other DomainParticipants
* Cryptographically check the domain access permissions of each DomainParticipant
* Encrypt and sign RTPS communications
* Security event logging
* Data tagging (not implemented in RustDDS)

Please see the [DDS Security Specification](https://www.omg.org/spec/DDS-SECURITY/1.1/About-DDS-SECURITY) v1.1 from OMG for technical details.

# Using security in RustDDS

In order to use the security functionality, enable the Cargo feature `security` in RustDDS. By default, it is not enabled, because it adds a large body of code and some processing overhead.

Security needs to be configured in order to be used. There are several mandatory configuration files that need to be supplied to RustDDS. These configuration files and their format and semantics are not unique to RustDDS, but specified in the OMG DDS Security specification. The security configuration files should also be interoperable between compliant DDS implementations.

Configuring security for DomainParticipants needs two Certificate Authority roles, or CAs. A CA is someone who has the ability to issue and sign the various configuration files. The two CAs are the Identity Certificate Authority and the Permissions Certificate Authority. 

It is possible that a single CA performs both of these roles. This is a matter of security configuration.

The job of the Identity CA is to issue and sign certificates that prove the identity of DomainParticipants. Each DomainParticipant must have their own identity.

The job of the Permissions CA is to sign permissions documents for the DomainParticipants. A permissions document defines which topics a DomainParticipant has read and/or write access.

The following security configuration files are needed:

## Identity CA Certificate

* Most important content is the CA's public key. It is used to verify whether Identity Certificates are actually signed by the CA.
* This is an X.509 Certificate `.pem` file.

## Participant Identity Certificate

* X.509 Certificate `.pem` file
* This file gives the Subject Name and corresponding public key for a DomainParticipant.
* Signed by the Identity CA.
* Not secret. Sent as plaintext to other DomainParticipants during authentication.

## Participant Private Key

* X.509 private key
* Secret, should be known only by the Participant it belongs to.
* Used to sign Authentication protocol messages to prove that we are the Subject Name stated in our Identity Certificate.

## Permissions CA Certificate

* Used to verify the authenticity of permissions documents, both our own and those presented to us over the authentication protocol.
* X.509 Certificate `.pem` file

## Participant Permissions

* An XML document giving domain access permissions to one or more Participants.
* XSD Schema given in the DDS Security Specification.
* Permissions allow/deny publish, subscribe, and/or relay access to various Topics in a Domain.
* Signed by Permissions CA.
* PKCS #7 signed document (`.p7s`) using S/MIME encoding.

## Domain Governance Document

* An XML document defining the domain-wide access rules not specific to any Participant.
  * Are unauthenticated participants allowed at all?
  * Is discovery protocol secured?
  * Is RTPS liveliness (heartbeat) messaging secured?
  * Topic-specific access rules, e.g. is data or metadata encrypted or signed, is reading or writing access controlled.
* XSD Schema given in the DDS Security Specification.
* Signed by Permissions CA.
* PKCS #7 signed document (`.p7s`) using S/MIME encoding.

# Creating configuration files

Configuration files can be created using any method, but the OpenSSL tool is recommended.

Please see the examples and scripts in the subdirectory [example_security_configuration_files](examples/security_configuration_files).
