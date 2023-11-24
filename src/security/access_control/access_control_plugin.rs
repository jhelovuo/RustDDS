use crate::{
  dds::qos::QosPolicies,
  discovery::{sedp_messages::TopicBuiltinTopicData, SpdpDiscoveredParticipantData},
  security::{authentication::*, *},
};
use super::*;

/// Access control plugin interface: section 8.4.2.9 of the Security
/// specification (v. 1.1).
///
/// To make use of Rust's features, the trait functions deviate a bit from the
/// specification. The main difference is that the functions return a Result
/// type. With this, there is no need to provide a pointer to a
/// SecurityException type which the function would fill in case of a failure.
/// Instead, the Err-variant of the result contains the error information. Also,
/// if a function has a single return value, it is returned inside the
/// Ok-variant. When a function returns a boolean according to the
/// specification, the Ok-variant is interpreted as true and Err-variant as
/// false.
///
/// We split the plugin interface to 3 parts according to the 5 groups described
/// in 8.8.3
pub trait AccessControl:
  ParticipantAccessControl + LocalEntityAccessControl + RemoteEntityAccessControl
{
}

/// Group1 in 8.8.3
pub trait ParticipantAccessControl: Send {
  /// validate_local_permissions: section 8.4.2.9.1 of the Security
  /// specification
  fn validate_local_permissions(
    &mut self,
    auth_plugin: &dyn Authentication,
    identity: IdentityHandle,
    domain_id: u16,
    participant_qos: &QosPolicies,
  ) -> SecurityResult<PermissionsHandle>;

  /// validate_remote_permissions: section 8.4.2.9.2 of the Security
  /// specification
  fn validate_remote_permissions(
    &mut self,
    auth_plugin: &dyn Authentication,
    local_identity_handle: IdentityHandle,
    remote_identity_handle: IdentityHandle,
    remote_permissions_token: &PermissionsToken,
    remote_credential_token: &AuthenticatedPeerCredentialToken,
  ) -> SecurityResult<PermissionsHandle>;

  /// check_create_participant: section 8.4.2.9.3 of the Security
  /// specification
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  fn check_create_participant(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    qos: &QosPolicies,
  ) -> SecurityResult<bool>;

  /// check_remote_participant: section 8.4.2.9.9 of the Security
  /// specification.
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  fn check_remote_participant(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    participant_data: Option<&SpdpDiscoveredParticipantData>,
  ) -> SecurityResult<bool>;

  /// get_permissions_token: section 8.4.2.9.17 of the Security
  /// specification.
  fn get_permissions_token(&self, handle: PermissionsHandle) -> SecurityResult<PermissionsToken>;

  /// get_permissions_credential_token: section 8.4.2.9.18 of the Security
  /// specification.
  fn get_permissions_credential_token(
    &self,
    handle: PermissionsHandle,
  ) -> SecurityResult<PermissionsCredentialToken>;

  /// set_listener: section 8.4.2.9.19 of the Security
  /// specification.
  /// TODO: we do not need this as listeners are not used in RustDDS, but which
  /// async mechanism to use?
  fn set_listener(&self) -> SecurityResult<()>;

  /// get_participant_sec_attributes: section 8.4.2.9.22 of the Security
  /// specification.
  fn get_participant_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
  ) -> SecurityResult<ParticipantSecurityAttributes>;
}

/// Group2 and Group3 in 8.8.3
pub trait LocalEntityAccessControl: Send {
  /// check_create_datawriter: section 8.4.2.9.4 of the Security
  /// specification. The parameters partition and data_tag have been left out,
  /// since RustDDS does not yet support PartitionQoS or data tagging
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  fn check_create_datawriter(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    qos: &QosPolicies,
  ) -> SecurityResult<bool>;

  /// check_create_datareader: section 8.4.2.9.5 of the Security
  /// specification. The parameters partition and data_tag have been left out,
  /// since RustDDS does not yet support PartitionQoS or data tagging
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  fn check_create_datareader(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    qos: &QosPolicies,
  ) -> SecurityResult<bool>;

  /// check_create_topic: section 8.4.2.9.6 of the Security
  /// specification
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  fn check_create_topic(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    qos: &QosPolicies,
  ) -> SecurityResult<bool>;

  /*
  /// check_local_datawriter_register_instance: section 8.4.2.9.7 of the
  /// Security specification.
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  // Support for this is not yet implemented as the builtin plugin does not need it
  fn check_local_datawriter_register_instance(
    &self,
    permissions_handle: PermissionsHandle,
    writer: DataWriter,// Needs actual type definition
    key: DynamicData,// Needs actual type definition
  ) -> SecurityResult<bool>;
  */

  /*
  /// check_local_datawriter_register_instance: section 8.4.2.9.8 of the
  /// Security specification.
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  // Support for this is not yet implemented as the builtin plugin does not need it
  fn check_local_datawriter_dispose_instance(
    &self,
    permissions_handle: PermissionsHandle,
    writer: DataWriter,// Needs actual type definition
    key: DynamicData,// Needs actual type definition
  ) -> SecurityResult<bool>;
  */

  /// get_topic_sec_attributes: section 8.4.2.9.23 of the Security
  /// specification.
  fn get_topic_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: &str,
  ) -> SecurityResult<TopicSecurityAttributes>;

  /// get_datawriter_sec_attributes: section 8.4.2.9.24 of the Security
  /// specification.
  /// The parameters partition and data_tag have been left out,
  /// since RustDDS does not yet support PartitionQoS or data tagging
  fn get_datawriter_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes>;

  /// get_datareader_sec_attributes: section 8.4.2.9.25 of the Security
  /// specification.
  /// The parameters partition and data_tag have been left out,
  /// since RustDDS does not yet support PartitionQoS or data tagging
  fn get_datareader_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes>;
}

/// Group4 and Group5 in 8.8.3
pub trait RemoteEntityAccessControl: Send {
  /// check_remote_datawriter: section 8.4.2.9.10 of the Security
  /// specification.
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  fn check_remote_datawriter(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    publication_data: &PublicationBuiltinTopicDataSecure,
  ) -> SecurityResult<bool>;

  /// check_remote_datareader: section 8.4.2.9.11 of the Security
  /// specification.
  /// In the returned Ok-variant, the first boolean tells if the remote
  /// DataReader passed the permission check. The second boolean is the
  /// relay_only value (see the spec)
  fn check_remote_datareader(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    subscription_data: &SubscriptionBuiltinTopicDataSecure,
  ) -> SecurityResult<(bool, bool)>;

  /// check_remote_topic: section 8.4.2.9.12 of the Security
  /// specification.
  /// In the returned Ok-variant, the boolean tells if the participant passed
  /// the permission check.
  fn check_remote_topic(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_data: &TopicBuiltinTopicData,
  ) -> SecurityResult<bool>;

  /*
  /// check_local_datawriter_match: section 8.4.2.9.13 of the Security
  /// specification.
  // Support for this is not yet implemented as the builtin plugin does not need it
  fn check_local_datawriter_match(
    &self,
    writer_permissions_handle: PermissionsHandle,
    reader_permissions_handle: PermissionsHandle,
    publication_data: &PublicationBuiltinTopicDataSecure,
    subscription_data: &SubscriptionBuiltinTopicDataSecure,
  ) -> SecurityResult<()>;
  */

  /*
   /// check_local_datareader_match: section 8.4.2.9.14 of the Security
   /// specification.
   /// The parameter subscriber_partition is omitted since RustDDS does not yet
   /// support PartitionQoS.
   // Support for this is not yet implemented as the builtin plugin does not need it
   fn check_local_datareader_match(
     &self,
     reader_permissions_handle: PermissionsHandle,
     writer_permissions_handle: PermissionsHandle,
     subscription_data: &SubscriptionBuiltinTopicDataSecure,
     publication_data: &PublicationBuiltinTopicDataSecure,
   ) -> SecurityResult<()>;
  */

  /*
  /// check_remote_datawriter_register_instance: section 8.4.2.9.15 of the
  /// Security specification.
  // Support for this is not yet implemented as the builtin plugin does not need
  // it
  fn check_remote_datawriter_register_instance(
    &self,
    permissions_handle: PermissionsHandle,
    reader: DataReader,                   // Needs actual type definition
    publication_handle: InstanceHandle_t, // Needs actual type definition
    key: DynamicData,                     // Needs actual type definition
    instance_handle: InstanceHandle_t,    // Needs actual type definition
  ) -> SecurityResult<()>;
  */

  /*
  /// check_remote_datawriter_dispose_instance: section 8.4.2.9.16 of the
  /// Security specification.
  // Support for this is not yet implemented as the builtin plugin does not need
  // it
  fn check_remote_datawriter_dispose_instance(
    &self,
    permissions_handle: PermissionsHandle,
    reader: DataReader,                   // Needs actual type definition
    publication_handle: InstanceHandle_t, // Needs actual type definition
    key: DynamicData,                     // Needs actual type definition
  ) -> SecurityResult<()>;
  */
}

// TODO: Can the different return methods (e.g. return_permissions_token) be
// left out, since Rust manages memory for us?
