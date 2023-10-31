# Security in RustDDS

Security support in DDS means the abilities to

* Cryptographically authenticate other DomainPArticipants
* Cryptographically check the domain acecss permisions of each DomainParticipant
* Encrypt and sign RTPS communications
* Security event logging
* Data tagging (not implemented in RustDDS)

Please see the [DDS Security Specification](https://www.omg.org/spec/DDS-SECURITY/1.1/About-DDS-SECURITY) v1.1 from OMG for technical details.

# Using security in RustDDS

In order to use the security functionality, use enable the Cargo feature `security` in RustDDS. By default, it is not enabled, because it adds a large body of code and some processing overhead.

Security needs to be confgured in order to be used. There are several mandatory configuration files that need to be supplied ot RustDDS. These configuration files and their format and semantics are not unique to RustDDS, but specified in the OMG DDS Security specification. The security configration files should also be interoperable between compliant DDS implementations.

Configuring security for DomainParticipants needs two Certification Authority roles, or CAs. A CA is someone who has the ability to issue and sign the various configuration files. The two CAs are the Identity Certification Authority and the Permissions Certificate Authority. 

It is possible that a single CA performs both of these roles. This is a matter of security configuration.


The job of the Identity CA is to issue and sign certificates that prove the identity of DomainParticipants. Each DomainParticiapnt must have their own identity.

The job of the Permissions CA is to sign permissions documents for the DomainParticipants. These documents are sets of rules 

The following security configuraiton files are needed:

## Identity CA Certificate

* Most important content is the CA's public key. It is used to verify signatures that should have been by the CA. 
* This is an X.509 Certificate `.pem` file.

## Participant Identity Certificate

* X.509 Certificate `.pem` file
* This file fives the Subject Name and corresponding public key for a DomainParticipant.
* Signed by Identity CA.
* Not secret. Sent as plaintext to other DomainParticiapnts during authentication.

## Participant Private Key

* X.509 private key
* Secret, should be known only by the PArticipant it belongs to.
* Used to sign Authentication protocol messages to prove that we are the Subject Name stated in our Identity Certificate.

## Permissions CA Certificate

* Used to verify the auhenticity of permisisons documents, both our own and those presented to us over the authentication protocol.
* X.509 Certificate (`.pem`)

## Participant Permissions

* An XML document giving domain access permissions to one or more Particiapnts.
* XSD Schema given in the DDS Security Specification.
* Permissions allow/deny publish, subscribe, and/or relay access to various Topics in a Domain.
* Signed by Permissions CA.
* PKCS #7 signed document (`.p7s`) using S/MIME encoding.

## Domain Governance Document

* An XML document defining the domain-wide acecss rules  not specific to any Participant.
  * Are unauthenticated participants allowed at all?
  * Is discovery protocol secured?
  * Is RTPS liveliness (heartbeat) messaging secured?
  * Topic-specific access rules, e.g. is data or metadata encrypted or signed, is reading or writing access controlled.
* XSD Schema given in the DDS Security Specification.
* Signed by Permissions CA.
* PKCS #7 signed document (`.p7s`) using S/MIME encoding.

# Creating configuration files

Configuration files can be created using any method, but the OpenSSL tool is recommended.

Please see the examples and scripts in the subdirectory [example_security_configuration_files](example_security_configuration_files/).
