use crate::{
  create_security_error_and_log,
  dds::qos::QosPolicies,
  rtps::constant::builtin_topic_names,
  security::{access_control::*, *},
};
use super::{
  domain_governance_document::{DomainRule, TopicRule},
  types::{BuiltinPluginEndpointSecurityAttributes, Entity},
};

impl AccessControlBuiltin {
  fn get_endpoint_security_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: &str,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    // Special handling for builtin topics
    match topic_name {
      // 7.4.8: is_submessage_protected shall match is_discovery_protected of the participant
      // security attributes
      builtin_topic_names::DCPS_PARTICIPANT_SECURE
      | builtin_topic_names::DCPS_PUBLICATIONS_SECURE
      | builtin_topic_names::DCPS_SUBSCRIPTIONS_SECURE => {
        self.get_domain_rule(&permissions_handle).map(
          |DomainRule {
             discovery_protection_kind,
             ..
           }| {
            let (
              is_submessage_protected,
              is_submessage_encrypted,
              is_submessage_origin_authenticated,
            ) = discovery_protection_kind.to_security_attributes_format();

            EndpointSecurityAttributes::for_builtin_topic(
              is_submessage_protected,
              is_submessage_encrypted,
              is_submessage_origin_authenticated,
            )
          },
        )
      }
      // 7.4.8: is_submessage_protected shall match is_liveliness_protected of the participant
      // security attributes
      builtin_topic_names::DCPS_PARTICIPANT_MESSAGE_SECURE => {
        self.get_domain_rule(&permissions_handle).map(
          |DomainRule {
             liveliness_protection_kind,
             ..
           }| {
            let (
              is_submessage_protected,
              is_submessage_encrypted,
              is_submessage_origin_authenticated,
            ) = liveliness_protection_kind.to_security_attributes_format();

            EndpointSecurityAttributes::for_builtin_topic(
              is_submessage_protected,
              is_submessage_encrypted,
              is_submessage_origin_authenticated,
            )
          },
        )
      }

      // This topic is for sharing keys. A unique encryption key is used for each receiver, so no
      // additional origin authentication is needed.
      builtin_topic_names::DCPS_PARTICIPANT_VOLATILE_MESSAGE_SECURE => Ok(
        EndpointSecurityAttributes::for_builtin_topic(true, true, false),
      ),

      // 7.4.8 for stateless, the others are used for normal unprotected discovery
      builtin_topic_names::DCPS_PARTICIPANT_STATELESS_MESSAGE
      | builtin_topic_names::DCPS_PARTICIPANT
      | builtin_topic_names::DCPS_PARTICIPANT_MESSAGE
      | builtin_topic_names::DCPS_PUBLICATION
      | builtin_topic_names::DCPS_SUBSCRIPTION
      | builtin_topic_names::DCPS_TOPIC => Ok(EndpointSecurityAttributes::empty()),

      // General case
      topic_name => self
        .get_domain_rule(&permissions_handle)
        .and_then(|domain_rule| {
          domain_rule.find_topic_rule(topic_name).ok_or_else(|| {
            create_security_error_and_log!(
              "Could not find a topic rule for the topic_name {topic_name}"
            )
          })
        })
        .map(
          |TopicRule {
             enable_discovery_protection,
             enable_liveliness_protection,
             enable_read_access_control,
             enable_write_access_control,
             metadata_protection_kind,
             data_protection_kind,
             ..
           }| {
            let (
              is_submessage_protected,
              is_submessage_encrypted,
              is_submessage_origin_authenticated,
            ) = metadata_protection_kind.to_security_attributes_format();
            let (is_payload_protected, is_payload_encrypted, is_key_protected) =
              data_protection_kind.to_security_attributes_format();
            EndpointSecurityAttributes {
              topic_security_attributes: TopicSecurityAttributes {
                is_read_protected: *enable_read_access_control,
                is_write_protected: *enable_write_access_control,
                is_discovery_protected: *enable_discovery_protection,
                is_liveliness_protected: *enable_liveliness_protection,
              },
              is_submessage_protected,
              is_payload_protected,
              is_key_protected,
              plugin_endpoint_attributes: BuiltinPluginEndpointSecurityAttributes {
                is_submessage_encrypted,
                is_submessage_origin_authenticated,
                is_payload_encrypted,
              }
              .into(),
              ac_endpoint_properties: Vec::new(),
            }
          },
        ),
    }
  }
}

impl LocalEntityAccessControl for AccessControlBuiltin {
  fn check_create_datawriter(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    _qos: &QosPolicies,
  ) -> SecurityResult<bool> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from PartitionQosPolicy
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from DataTagQosPolicy
    self.check_entity(
      permissions_handle,
      domain_id,
      &topic_name,
      partitions,
      data_tags,
      &Entity::Datawriter,
    )
  }

  fn check_create_datareader(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    _qos: &QosPolicies,
  ) -> SecurityResult<bool> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from PartitionQosPolicy
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from DataTagQosPolicy
    self.check_entity(
      permissions_handle,
      domain_id,
      &topic_name,
      partitions,
      data_tags,
      &Entity::Datareader,
    )
  }

  fn check_create_topic(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    _qos: &QosPolicies,
  ) -> SecurityResult<bool> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from PartitionQosPolicy
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from DataTagQosPolicy
    self.check_entity(
      permissions_handle,
      domain_id,
      &topic_name,
      partitions,
      data_tags,
      &Entity::Topic,
    )
  }
  /*
  // Support for this is not yet implemented as the builtin plugin does not need
  // it
  fn check_local_datawriter_register_instance(
    &self,
    _permissions_handle: PermissionsHandle,
    _writer: DataWriter, // Needs actual type definition
    _key: DynamicData,   // Needs actual type definition
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }
  */

  /*
  // Support for this is not yet implemented as the builtin plugin does not need
  // it
  fn check_local_datawriter_dispose_instance(
    &self,
    _permissions_handle: PermissionsHandle,
    _writer: DataWriter, // Needs actual type definition
    _key: DynamicData,   // Needs actual type definition
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }
  */

  // Currently only mocked, but ready after removing the last line
  fn get_topic_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: &str,
  ) -> SecurityResult<TopicSecurityAttributes> {
    self
      .get_endpoint_security_attributes(permissions_handle, topic_name)
      .map(
        |EndpointSecurityAttributes {
           topic_security_attributes,
           ..
         }| topic_security_attributes,
      )
  }

  // Currently only mocked, but ready after removing the last line
  fn get_datawriter_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    self.get_endpoint_security_attributes(permissions_handle, &topic_name)
  }

  // Currently only mocked, but ready after removing the last line
  fn get_datareader_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    self.get_endpoint_security_attributes(permissions_handle, &topic_name)
  }
}
