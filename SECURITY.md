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

**Note**: if RustDDS needs to interoperate with DomainParticipants based on [FastDDS](https://github.com/eProsima/Fast-DDS), enable the feature `security_in_fastdds_compatibility_mode` instead. For more info, see the section [Security in FastDDS-compatible mode](#security-in-fastdds-compatible-mode) below.



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

Please see the examples and scripts in the directory [examples/security_configuration_files](examples/security_configuration_files).


# Security in FastDDS-compatible mode

Currently, our default DDS Security implementation is not compatible with the implementation in FastDDS. Because of this, RustDDS compiled with the default security implementation cannot establish a secure connection with a DomainParticipant based on FastDDS. The problem here is that, at least from our perspective, the FastDDS implementation slightly deviates from the Security specification at some points.

To remedy this, we have created the feature `security_in_fastdds_compatibility_mode`. When RustDDS is compiled with this feature enabled, the security functionality is adjusted to obtain interoperability with FastDDS. The section below describes these adjustments.

**Note**: Because FastDDS is a widely used DDS implementation that likely interoperates with some other DDS implementations, it might be that the feature `security_in_fastdds_compatibility_mode` is required to obtain interoperability also with some other DDS implementation.

## Adjustments to Security implementation in the compatibility mode

### 1. In PermissionsToken and IdentityToken, use the algorithm identifiers expected by FastDDS
- The DDS Security specification (section 9.3.2.1 DDS:Auth:PKI-DH IdentityToken) states that In PermissionsToken and IdentityToken, the algorithm identifier (a string) is either `RSA-2048` or `EC-prime256v1`
- FastDDS uses the identifiers ``RSASSA-PSS-SHA256`` and ``ECDSA-SHA256`` which are similar, but used elsewhere in the spec.
- Because of this, FastDDS does not recognize the algorithm identifiers sent by RustDDS, and refuses to establish a connection

**Compatiblity fix** ✅: Use the identifiers that FastDDS expects also in RustDDS

**Proper fix**: Use the correct identifiers in FastDDS. TODO: submit an issue about it to FastDDS.

### 2. Use the same string representation of certificate Subject name as FastDDS
- To represent a certificate’s Subject Name as a string, FastDDS uses the OpenSSL function [X509_NAME_oneline](https://www.openssl.org/docs/manmaster/man3/X509_NAME_oneline.html). In the OpenSSL documentation, this function is said to produce a non-standard output, and “its use is strongly discouraged in new applications and it could be deprecated in a future release”.
- RustDDS uses the [string formatting from the x509_cert crate](https://docs.rs/x509-cert/latest/x509_cert/name/struct.RdnSequence.html#impl-Display-for-RdnSequence) , which produces a representation according to the [RFC 4514](https://datatracker.ietf.org/doc/html/rfc4514), which seems to be the standard nowadays. A function equivalent to the X509_NAME_oneline used by FastDDS is not available in the openssl or certificate crates that RustDDS is using. Such a function seems to be available in the [boring_sys](https://docs.rs/boring-sys/latest/boring_sys/fn.X509_NAME_oneline.html) crate, but using this would require bringing in a complex new crate just for this quite small functionality & tinkering with unsafe code.
- The problem arises when FastDDS compares the CA subject name string (standard) that RustDDS has sent inside a PermissionsToken to its own string (non-standard) that FastDDS has computed from the permissions CA certificate. Since these two strings don’t match, FastDDS deduces that it does not know the CA that RustDDS is using and refuses to establish a connection.
  - Note: ideally the Subject Name comparison method would be more robust than a simple string comparison
- Example formats:
  - RustDDS (RFC 4514): ``CN=permissions_ca_common_name,O=Example Organization``
  - FastDDS (X509_NAME_oneline): ``/O=Example Organization/CN=permissions_ca_common_name``


**Compatiblity fix** ✅: In RustDDS, we use a function for converting the subject name string from the standard format to the X509_NAME_oneline format expected by FastDDS. The conversion isn’t perfect (according to OpenSSL documentation, X509_NAME_oneline “has various quirks and inconsistencies”), but it should work in simple cases.

**Proper fix**: Does FastDDS even need to compare the CA Subject Name we send in PermissionsToken to its own version of the Subject Name? The spec says that the subject name field in the PermissionsToken is optional, and does not require checking it in any way. What the spec *does* require is that the signed permissions document sent by RustDDS is verified against the local permissions CA cerificate. Doesn't this already guarantee that we're using the same permissions CA?
