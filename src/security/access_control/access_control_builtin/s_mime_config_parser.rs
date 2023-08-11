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


use bytes::{Bytes};

use super::domain_participant_permissions_document::{ConfigError, to_config_error, config_error};

use der::Decode;
use cms::signed_data::{SignedData,EncapsulatedContentInfo,};

#[derive(Debug)]
pub struct SignedDocument {
  pub content: Bytes,
  pub signature: SignedData,
}

impl SignedDocument {
  pub fn from_reader(input: &[u8]) -> Result<SignedDocument, ConfigError> {

    let parsed_mail = mailparse::parse_mail(input)
      .map_err(|e| to_config_error("S/MIME read failure", e))?;

    match parsed_mail.subparts.as_slice() {
      [doc_content, signature] => {
        let b = Bytes::from(signature.get_body_raw()
              .map_err(|e| to_config_error("S/MIME signature read failure", e))?);
        let signature_encap = EncapsulatedContentInfo::from_der(&b).unwrap();
        let signature_oid = const_oid::ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
        let signature = 
          if signature_encap.econtent_type == signature_oid {
            // it is a signature
            match signature_encap.econtent {
              None => Err(config_error("Empty signature container??")),
              Some(sig) => 
                sig.decode_as::<SignedData>()
                  .map_err(|e| to_config_error("Signature contaner decode failed", e))
            }
          } else {
            Err(config_error(&format!("Expected signature OID, found {} instead",
             signature_encap.econtent_type)))
          } ? ;

        Ok(SignedDocument {
          content: Bytes::from(doc_content.get_body_raw()
            .map_err(|e| to_config_error("S/MIME content read failure", e))?),
          signature, 
        })
      }
      parts => 
        Err(config_error(&format!("Expected 2-part S/MIME document, found {} parts.", parts.len())))
    }
  }
}



#[cfg(test)]
mod tests {
  use super::*;
  use super::super::domain_governance_document::*;

  #[test]
  pub fn parse_example() {
    let domain_governance_document = r#"MIME-Version: 1.0
Content-Type: multipart/signed; protocol="application/x-pkcs7-signature"; micalg="sha-256"; boundary="----033B4A46DD7C9B214FE531BA256FE14E"

This is an S/MIME signed message

------033B4A46DD7C9B214FE531BA256FE14E
Content-Type: text/plain

<dds xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="http://www.omg.org/spec/DDS-SECURITY/20170901/omg_shared_ca_governance.xsd">
    <domain_access_rules>
        <domain_rule>
            <domains>
              <id>0</id>
            </domains>
            <allow_unauthenticated_participants>false</allow_unauthenticated_participants>
            <enable_join_access_control>true</enable_join_access_control>
            <discovery_protection_kind>ENCRYPT</discovery_protection_kind>
            <liveliness_protection_kind>ENCRYPT</liveliness_protection_kind>
            <rtps_protection_kind>SIGN</rtps_protection_kind>
            <topic_access_rules>
                <topic_rule>
                    <topic_expression>*</topic_expression>
                    <enable_discovery_protection>true</enable_discovery_protection>
                    <enable_liveliness_protection>true</enable_liveliness_protection>
                    <enable_read_access_control>true</enable_read_access_control>
                    <enable_write_access_control>true</enable_write_access_control>
                    <metadata_protection_kind>ENCRYPT</metadata_protection_kind>
                    <data_protection_kind>ENCRYPT</data_protection_kind>
                </topic_rule>
            </topic_access_rules>
        </domain_rule>
    </domain_access_rules>
</dds>

------033B4A46DD7C9B214FE531BA256FE14E
Content-Type: application/x-pkcs7-signature; name="smime.p7s"
Content-Transfer-Encoding: base64
Content-Disposition: attachment; filename="smime.p7s"

MIIC+AYJKoZIhvcNAQcCoIIC6TCCAuUCAQExDzANBglghkgBZQMEAgEFADALBgkq
hkiG9w0BBwGgggE/MIIBOzCB4qADAgECAhR361786/qVPfJWWDw4Wg5cmJUwBTAK
BggqhkjOPQQDAjASMRAwDgYDVQQDDAdzcm9zMkNBMB4XDTIzMDcyMzA4MjgzNloX
DTMzMDcyMTA4MjgzNlowEjEQMA4GA1UEAwwHc3JvczJDQTBZMBMGByqGSM49AgEG
CCqGSM49AwEHA0IABMpvJQ/91ZqnmRRteTL2qaEFz2d7SGAQQk9PIhhZCV1tlLwY
f/hI4xWLJaEv8FxJTjxXRGJ1U+/IqqqIvJVpWaSjFjAUMBIGA1UdEwEB/wQIMAYB
Af8CAQEwCgYIKoZIzj0EAwIDSAAwRQIgEiyVGRc664+/TE/HImA4WNwsSi/alHqP
YB58BWINj34CIQDDiHhbVPRB9Uxts9CwglxYgZoUdGUAxreYIIaLO4yLqzGCAX0w
ggF5AgEBMCowEjEQMA4GA1UEAwwHc3JvczJDQQIUd+te/Ov6lT3yVlg8OFoOXJiV
MAUwDQYJYIZIAWUDBAIBBQCggeQwGAYJKoZIhvcNAQkDMQsGCSqGSIb3DQEHATAc
BgkqhkiG9w0BCQUxDxcNMjMwNzI0MDgyODM2WjAvBgkqhkiG9w0BCQQxIgQgTnFs
6hUKJgZBcNbGYTTEAFkvkPyRGdLCHkwcepUDARIweQYJKoZIhvcNAQkPMWwwajAL
BglghkgBZQMEASowCwYJYIZIAWUDBAEWMAsGCWCGSAFlAwQBAjAKBggqhkiG9w0D
BzAOBggqhkiG9w0DAgICAIAwDQYIKoZIhvcNAwICAUAwBwYFKw4DAgcwDQYIKoZI
hvcNAwICASgwCgYIKoZIzj0EAwIERjBEAiA+R4rlznH3B4JZ3SodwZepKCz+EWnx
bJ7qobUT6gJXBgIgeHhk2Yp/9dawfziYeWZ9TlCcT4hZxBaENkfBQbl2Vz8=

------033B4A46DD7C9B214FE531BA256FE14E--

"#;

    let dgd_signed = SignedDocument::from_reader(&mut domain_governance_document.as_bytes() ) .unwrap();

    let dgd = DomainGovernanceDocument::from_xml(&String::from_utf8_lossy(dgd_signed.content.as_ref())).unwrap();

    println!("{:?}\n\n{:?}",dgd, dgd_signed.signature);

  }
}

