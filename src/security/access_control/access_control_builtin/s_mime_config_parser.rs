//
// DDS Security spec v1.1
// 
// Section "9.4.1.2 Domain Governance Document
// 
// The domain governance document is an XML document that specifies how the domain should be secured.
//
// The domain governance document shall be signed by the Permissions CA. The
// signed document shall use S/MIME version 3.2 format as defined in IETF RFC
// 5761 using SignedData Content Type (section 2.4.2 of IETF RFC 5761)
// formatted as multipart/signed (section 3.4.3 of IETF RFC 5761). This
// corresponds to the mime-type application/pkcs7-signature. Additionally the
// signer certificate shall be included within the signature."
//

// This module is for decoding the said S/MIME encoding. The same encoding applies also for
// the DomainParticipant Permissions Document. (Section 9.4.1.3)

use std::io::Write;
use std::process::Command;

use bytes::{Bytes};

use x509_certificate::certificate::{CapturedX509Certificate};

use super::domain_participant_permissions_document::{ConfigError, to_config_error, config_error};


#[derive(Debug)]
pub struct SignedDocument {
  input_bytes: Bytes,
  content: Bytes,
  signature_der: Bytes,
}

impl SignedDocument {
  pub fn from_bytes(input: &[u8]) -> Result<SignedDocument, ConfigError> {

    let parsed_mail = mailparse::parse_mail(input)
      .map_err(|e| to_config_error("S/MIME parse failure", e))?;

    match parsed_mail.subparts.as_slice() {
      [doc_content, signature] => {
        let content = Bytes::from(doc_content.get_body_raw()
              .map_err(|e| to_config_error("S/MIME content read failure", e))?);

        let signature_der = Bytes::from(signature.get_body_raw()
              .map_err(|e| to_config_error("S/MIME signature read failure", e))?);

        Ok(SignedDocument {
          input_bytes: Bytes::copy_from_slice(input),
          content,
          signature_der, 
        })
      }
      parts => 
        Err(config_error(&format!("Expected 2-part S/MIME document, found {} parts.", parts.len())))
    }
  }

  // Use given X.509 certificate (in PEM format) to verify signature 
  // and check that the data matches the signature.
  //
  // If successful, returns reference to the verified document.
  pub fn verify_signature(&self, certificate_pem: impl AsRef<[u8]>) 
    -> Result< impl AsRef<[u8]>, ConfigError> 
  {
    let cert = CapturedX509Certificate::from_pem(certificate_pem.as_ref())
      .map_err(|e| to_config_error("Cannot read X.509 Certificate",e) ) ?;

    let verification_result = 
      cert.verify_signed_data(self.content.as_ref(), self.signature_der.as_ref())
        .map_err(|e| to_config_error("SignedDocument: verification failure", e))
        .map( |()| self.content.clone() );

    // TODO:
    // The following is a backup logic, in case x509-certificate crate fails to verify.
    // Remove this after x509-certificate works as required, and just return `verification_result`.
    verification_result
      .or_else(|_e| self.verify_with_openssl(certificate_pem) )
  }

  fn verify_with_openssl(&self, certificate_pem: impl AsRef<[u8]>) -> Result<Bytes, ConfigError>
  {
    let mut doc_file = tempfile::NamedTempFile::new()
      .map_err(|e| to_config_error("Cannot open temp file 1", e))?;
    doc_file.write_all(self.input_bytes.as_ref())
      .map_err(|e| to_config_error("Cannot write temp file 1", e))?;
    

    let mut cert_file = tempfile::NamedTempFile::new()
      .map_err(|e| to_config_error("Cannot open temp file 2", e))?;
    cert_file.write_all(certificate_pem.as_ref())
      .map_err(|e| to_config_error("Cannot write temp file 2", e))?;

    let openssl_output = 
        Command::new("openssl")
            .args(["smime", "-verify", "-text",  "-in"])
            .arg(doc_file.path())
            .arg("-CAfile")
            .arg(cert_file.path())
            .output()
            .map_err(|e| to_config_error("Cannot excute openssl", e))?;

    if openssl_output.status.success() {
      Ok( self.content.clone() )
    } else {
      Err( config_error("Signature verification failed") )
    }

  } 


  // This is for test use only.
  // Use `verify_signature()` to get contents in a more secure manner.
  pub fn get_content_unverified(&self) -> impl AsRef<[u8]> {
    self.content.clone() // cheap Bytes clone
  }
}



#[cfg(test)]
mod tests {
  use super::*;
  use super::super::domain_governance_document::*;
  use super::super::domain_participant_permissions_document::*;

  #[test]
  pub fn parse_example() {
    let document = r#"MIME-Version: 1.0
Content-Type: multipart/signed; protocol="application/x-pkcs7-signature"; micalg="sha-256"; boundary="----D7B1B9F5CFE597B428D44692C7027BCF"

This is an S/MIME signed message

------D7B1B9F5CFE597B428D44692C7027BCF
Content-Type: text/plain

<dds xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="http://www.omg.org/spec/DDS-SECURITY/20170901/omg_shared_ca_permissions.xsd">
  <permissions>
    <grant name="/talker/api">
      <subject_name>CN=/talker/api</subject_name>
      <validity>
        <not_before>2023-07-23T08:28:37</not_before>
        <not_after>2033-07-21T08:28:37</not_after>
      </validity>
      <allow_rule>
        <domains>
          <id>0</id>
        </domains>
        <publish>
          <topics>
            <topic>rq/*/_action/cancel_goalRequest</topic>
            <topic>rq/*/_action/get_resultRequest</topic>
            <topic>rq/*/_action/send_goalRequest</topic>
            <topic>rq/*Request</topic>
            <topic>rr/*/_action/cancel_goalReply</topic>
            <topic>rr/*/_action/get_resultReply</topic>
            <topic>rr/*/_action/send_goalReply</topic>
            <topic>rt/*/_action/feedback</topic>
            <topic>rt/*/_action/status</topic>
            <topic>rr/*Reply</topic>
            <topic>rt/*</topic>
          </topics>
        </publish>
        <subscribe>
          <topics>
            <topic>rq/*/_action/cancel_goalRequest</topic>
            <topic>rq/*/_action/get_resultRequest</topic>
            <topic>rq/*/_action/send_goalRequest</topic>
            <topic>rq/*Request</topic>
            <topic>rr/*/_action/cancel_goalReply</topic>
            <topic>rr/*/_action/get_resultReply</topic>
            <topic>rr/*/_action/send_goalReply</topic>
            <topic>rt/*/_action/feedback</topic>
            <topic>rt/*/_action/status</topic>
            <topic>rr/*Reply</topic>
            <topic>rt/*</topic>
          </topics>
        </subscribe>
      </allow_rule>
      <allow_rule>
        <domains>
          <id>0</id>
        </domains>
        <publish>
          <topics>
            <topic>ros_discovery_info</topic>
          </topics>
        </publish>
        <subscribe>
          <topics>
            <topic>ros_discovery_info</topic>
          </topics>
        </subscribe>
      </allow_rule>
      <default>DENY</default>
    </grant>
  </permissions>
</dds>

------D7B1B9F5CFE597B428D44692C7027BCF
Content-Type: application/x-pkcs7-signature; name="smime.p7s"
Content-Transfer-Encoding: base64
Content-Disposition: attachment; filename="smime.p7s"

MIIC+QYJKoZIhvcNAQcCoIIC6jCCAuYCAQExDzANBglghkgBZQMEAgEFADALBgkq
hkiG9w0BBwGgggE/MIIBOzCB4qADAgECAhR361786/qVPfJWWDw4Wg5cmJUwBTAK
BggqhkjOPQQDAjASMRAwDgYDVQQDDAdzcm9zMkNBMB4XDTIzMDcyMzA4MjgzNloX
DTMzMDcyMTA4MjgzNlowEjEQMA4GA1UEAwwHc3JvczJDQTBZMBMGByqGSM49AgEG
CCqGSM49AwEHA0IABMpvJQ/91ZqnmRRteTL2qaEFz2d7SGAQQk9PIhhZCV1tlLwY
f/hI4xWLJaEv8FxJTjxXRGJ1U+/IqqqIvJVpWaSjFjAUMBIGA1UdEwEB/wQIMAYB
Af8CAQEwCgYIKoZIzj0EAwIDSAAwRQIgEiyVGRc664+/TE/HImA4WNwsSi/alHqP
YB58BWINj34CIQDDiHhbVPRB9Uxts9CwglxYgZoUdGUAxreYIIaLO4yLqzGCAX4w
ggF6AgEBMCowEjEQMA4GA1UEAwwHc3JvczJDQQIUd+te/Ov6lT3yVlg8OFoOXJiV
MAUwDQYJYIZIAWUDBAIBBQCggeQwGAYJKoZIhvcNAQkDMQsGCSqGSIb3DQEHATAc
BgkqhkiG9w0BCQUxDxcNMjMwNzI0MDgyODM3WjAvBgkqhkiG9w0BCQQxIgQgZMN5
ePtmuncArHGyik6XN2V2jXQfvALMN5VW3lfTBEYweQYJKoZIhvcNAQkPMWwwajAL
BglghkgBZQMEASowCwYJYIZIAWUDBAEWMAsGCWCGSAFlAwQBAjAKBggqhkiG9w0D
BzAOBggqhkiG9w0DAgICAIAwDQYIKoZIhvcNAwICAUAwBwYFKw4DAgcwDQYIKoZI
hvcNAwICASgwCgYIKoZIzj0EAwIERzBFAiEAlWvUBFIygrtc7cPc5aWp7RUBKATm
xIIOy2nxSv6UekICIEoA39P0Va694kOj2FSR2CqInHrmGghkuuR5au8Aoaab

------D7B1B9F5CFE597B428D44692C7027BCF--

"#;

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

    let dpp_signed = SignedDocument::from_bytes(&mut document.as_bytes() ) .unwrap();

    let verified_dpp_xml = dpp_signed.verify_signature(cert_pem).unwrap();

    let dpp = DomainParticipantPermissions::from_xml(&String::from_utf8_lossy( verified_dpp_xml.as_ref() )).unwrap();

    // getting here with no panic is success

    //println!("{:?}", dpp);
  }

  #[test]
  pub fn parse_small_example() {
    let document = r#"MIME-Version: 1.0
Content-Type: multipart/signed; protocol="application/x-pkcs7-signature"; micalg="sha-256"; boundary="----26B97E9080C33B5E9E82D8FDC0946E23"

This is an S/MIME signed message

------26B97E9080C33B5E9E82D8FDC0946E23
Content-Type: text/plain

<?xml version="1.0" encoding="utf-8"?>
<dds xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
xsi:noNamespaceSchemaLocation="http://www.omg.org/spec/DDS-Security/20170801/omg_shared_ca_domain_governance.xsd">
  <domain_access_rules>
    <domain_rule>
      <domains>
        <id>0</id>
      </domains>

      <allow_unauthenticated_participants>false</allow_unauthenticated_participants>
      <enable_join_access_control>true</enable_join_access_control>
      <rtps_protection_kind>SIGN</rtps_protection_kind>
      <discovery_protection_kind>SIGN</discovery_protection_kind>
      <liveliness_protection_kind>SIGN</liveliness_protection_kind>

      <topic_access_rules>
        <topic_rule>
          <topic_expression>Square*</topic_expression>
          <enable_discovery_protection>true
          </enable_discovery_protection>
          <enable_liveliness_protection>false</enable_liveliness_protection>
          <enable_read_access_control>true
          </enable_read_access_control>
          <enable_write_access_control>true
          </enable_write_access_control>
          <metadata_protection_kind>ENCRYPT
          </metadata_protection_kind>
          <data_protection_kind>ENCRYPT
          </data_protection_kind>
        </topic_rule>
      </topic_access_rules>
    </domain_rule>
  </domain_access_rules>
</dds>

------26B97E9080C33B5E9E82D8FDC0946E23
Content-Type: application/x-pkcs7-signature; name="smime.p7s"
Content-Transfer-Encoding: base64
Content-Disposition: attachment; filename="smime.p7s"

MIIC+QYJKoZIhvcNAQcCoIIC6jCCAuYCAQExDzANBglghkgBZQMEAgEFADALBgkq
hkiG9w0BBwGgggE/MIIBOzCB4qADAgECAhR361786/qVPfJWWDw4Wg5cmJUwBTAK
BggqhkjOPQQDAjASMRAwDgYDVQQDDAdzcm9zMkNBMB4XDTIzMDcyMzA4MjgzNloX
DTMzMDcyMTA4MjgzNlowEjEQMA4GA1UEAwwHc3JvczJDQTBZMBMGByqGSM49AgEG
CCqGSM49AwEHA0IABMpvJQ/91ZqnmRRteTL2qaEFz2d7SGAQQk9PIhhZCV1tlLwY
f/hI4xWLJaEv8FxJTjxXRGJ1U+/IqqqIvJVpWaSjFjAUMBIGA1UdEwEB/wQIMAYB
Af8CAQEwCgYIKoZIzj0EAwIDSAAwRQIgEiyVGRc664+/TE/HImA4WNwsSi/alHqP
YB58BWINj34CIQDDiHhbVPRB9Uxts9CwglxYgZoUdGUAxreYIIaLO4yLqzGCAX4w
ggF6AgEBMCowEjEQMA4GA1UEAwwHc3JvczJDQQIUd+te/Ov6lT3yVlg8OFoOXJiV
MAUwDQYJYIZIAWUDBAIBBQCggeQwGAYJKoZIhvcNAQkDMQsGCSqGSIb3DQEHATAc
BgkqhkiG9w0BCQUxDxcNMjMwODE0MDc1NDQxWjAvBgkqhkiG9w0BCQQxIgQgvAEn
eveae5s8eNw0HdhAcxPkLpHI3cdiMLrX5U6hwNoweQYJKoZIhvcNAQkPMWwwajAL
BglghkgBZQMEASowCwYJYIZIAWUDBAEWMAsGCWCGSAFlAwQBAjAKBggqhkiG9w0D
BzAOBggqhkiG9w0DAgICAIAwDQYIKoZIhvcNAwICAUAwBwYFKw4DAgcwDQYIKoZI
hvcNAwICASgwCgYIKoZIzj0EAwIERzBFAiAZQGxjfAoLlk99UWV5AYkHr1CGvOrn
X/iBDEnMibF4NAIhAPB45KRXnnC8QmjYByycsOo4uGDrrUZ4K+tWLBfOv8v9

------26B97E9080C33B5E9E82D8FDC0946E23--
"#;
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

    let dgd_signed = SignedDocument::from_bytes(&mut document.as_bytes() ) .unwrap();

    let verified_dgd_xml = dgd_signed.verify_signature(cert_pem).unwrap();

    let dgd = DomainGovernanceDocument::from_xml(&String::from_utf8_lossy( verified_dgd_xml.as_ref() )).unwrap();

    // getting here with no panic is success

    //println!("{:?}", dgd);
  }


}

