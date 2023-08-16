//
// DDS Security spec v1.1
//
// Section "9.4.1.1 Permissions CA Certificate
//
// This is an X.509 certificate that contains the Public Key of the CA that will
// be used to sign the Domain Governance and Domain Permissions document. The
// certificate can be self-signed or signed by some other CA. Regardless of
// this the Public Key in the Certificate shall be trusted to sign the
// aforementioned Governance and Permissions documents (see 9.4.1.2 and
// 9.4.1.3). The Permissions CA Certificate shall be provided to the plugins
// using the PropertyQosPolicy on the DomainParticipantQos as specified in
// Table 56."
//

use x509_certificate::certificate::CapturedX509Certificate;

use super::domain_participant_permissions_document::{to_config_error, ConfigError};

#[derive(Debug)]
struct Certificate {
  cert: CapturedX509Certificate,
}

impl Certificate {
  pub fn from_pem(pem_data: impl AsRef<[u8]>) -> Result<Self, ConfigError> {
    let cert = CapturedX509Certificate::from_pem(pem_data)
      .map_err(|e| to_config_error("Cannot read X.509 Certificate", e))?;

    Ok(Certificate { cert })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn parse_example() {
    let cert_pem = r#"-----BEGIN CERTIFICATE-----
MIIBOzCB4qADAgECAhR361786/qVPfJWWDw4Wg5cmJUwBTAKBggqhkjOPQQDAjAS
MRAwDgYDVQQDDAdzcm9zMkNBMB4XDTIzMDcyMzA4MjgzNloXDTMzMDcyMTA4Mjgz
NlowEjEQMA4GA1UEAwwHc3JvczJDQTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IA
BMpvJQ/91ZqnmRRteTL2qaEFz2d7SGAQQk9PIhhZCV1tlLwYf/hI4xWLJaEv8FxJ
TjxXRGJ1U+/IqqqIvJVpWaSjFjAUMBIGA1UdEwEB/wQIMAYBAf8CAQEwCgYIKoZI
zj0EAwIDSAAwRQIgEiyVGRc664+/TE/HImA4WNwsSi/alHqPYB58BWINj34CIQDD
iHhbVPRB9Uxts9CwglxYgZoUdGUAxreYIIaLO4yLqw==
-----END CERTIFICATE-----
"#;

    let cert = Certificate::from_pem(cert_pem).unwrap();

    println!("{:?}", cert);
  }
}
