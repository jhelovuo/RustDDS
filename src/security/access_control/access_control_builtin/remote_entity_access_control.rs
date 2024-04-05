use std::ops::Not;

use crate::{
  discovery::{
    sedp_messages::TopicBuiltinTopicData, DiscoveredReaderData, DiscoveredWriterData,
    PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
  },
  security::{access_control::*, *},
};
use super::{
  domain_governance_document::TopicRule, domain_participant_permissions_document::Action,
  types::Entity,
};

impl RemoteEntityAccessControl for AccessControlBuiltin {
  fn check_remote_datawriter(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    publication_data: &PublicationBuiltinTopicDataSecure,
  ) -> SecurityResult<bool> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from publication_data
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from publication_data

    let PublicationBuiltinTopicDataSecure {
      discovered_writer_data:
        DiscoveredWriterData {
          publication_topic_data: PublicationBuiltinTopicData { topic_name, .. },
          ..
        },
      ..
    } = publication_data;

    // Move the following check to validate_remote_permissions from check_remote_
    // methods, as there we have access to the tokens: "If the PluginClassName
    // or the MajorVersion of the local permissions_token differ from those in
    // the remote_permissions_token, the operation shall return FALSE."

    self.check_entity(
      permissions_handle,
      domain_id,
      topic_name,
      partitions,
      data_tags,
      &Entity::Datawriter,
    )
  }

  // Currently only mocked
  fn check_remote_datareader(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    subscription_data: &SubscriptionBuiltinTopicDataSecure,
  ) -> SecurityResult<(bool, bool)> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from publication_data
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from publication_data

    let SubscriptionBuiltinTopicDataSecure {
      discovered_reader_data:
        DiscoveredReaderData {
          subscription_topic_data: SubscriptionBuiltinTopicData { topic_name, .. },
          ..
        },
      ..
    } = subscription_data;

    // This method differs from the other similar ones because of the possibility of
    // a relay only datareader

    let grant = self.get_grant(&permissions_handle)?;
    let domain_rule = self.get_domain_rule(&permissions_handle)?;

    let requested_access_is_unprotected = domain_rule
      .find_topic_rule(topic_name)
      .map(
        |TopicRule {
           enable_read_access_control,
           ..
         }| *enable_read_access_control,
      )
      .is_some_and(bool::not);

    let participant_has_read_access = grant
      .check_action(
        Action::Subscribe,
        domain_id,
        topic_name,
        partitions,
        data_tags,
      )
      .into();

    // Move the following check to validate_remote_permissions from check_remote_
    // methods, as there we have access to the tokens: "If the PluginClassName
    // or the MajorVersion of the local permissions_token differ from those in
    // the remote_permissions_token, the operation shall return FALSE."

    let allow_to_fully_read = requested_access_is_unprotected || participant_has_read_access;

    let relay_only = if allow_to_fully_read {
      // Participant allowed to fully read the topic, relay_only has no meaning
      false
    } else {
      // Participant is not allowed to fully read the topic. But is it allowed to
      // relay it?
      bool::from(grant.check_action(Action::Relay, domain_id, topic_name, partitions, data_tags))
    };

    // check_passed = true means that participant is allowed to either fully read
    // the topic or relay it.
    let check_passed = allow_to_fully_read || relay_only;
    Ok((check_passed, relay_only))
  }

  fn check_remote_topic(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_data: &TopicBuiltinTopicData,
  ) -> SecurityResult<bool> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from publication_data
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from publication_data

    let TopicBuiltinTopicData { name, .. } = topic_data;

    // Move the following check to validate_remote_permissions from check_remote_
    // methods, as there we have access to the tokens: "If the PluginClassName
    // or the MajorVersion of the local permissions_token differ from those in
    // the remote_permissions_token, the operation shall return FALSE."

    self.check_entity(
      permissions_handle,
      domain_id,
      name,
      partitions,
      data_tags,
      &Entity::Topic,
    )
  }

  /*
  // Support for this is not yet implemented as the builtin plugin does not need
  // it
  fn check_local_datawriter_match(
    &self,
    _writer_permissions_handle: PermissionsHandle,
    _reader_permissions_handle: PermissionsHandle,
    _publication_data: &PublicationBuiltinTopicDataSecure,
    _subscription_data: &SubscriptionBuiltinTopicDataSecure,
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }
  */

  /*
  // Support for this is not yet implemented as the builtin plugin does not need
  // it
  fn check_local_datareader_match(
    &self,
    _reader_permissions_handle: PermissionsHandle,
    _writer_permissions_handle: PermissionsHandle,
    _subscription_data: &SubscriptionBuiltinTopicDataSecure,
    _publication_data: &PublicationBuiltinTopicDataSecure,
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }
  */

  /*
  // Support for this is not yet implemented as the builtin plugin does not need
  // it
  fn check_remote_datawriter_register_instance(
    &self,
    _permissions_handle: PermissionsHandle,
    _reader: DataReader,                   // Needs actual type definition
    _publication_handle: InstanceHandle_t, // Needs actual type definition
    _key: DynamicData,                     // Needs actual type definition
    _instance_handle: InstanceHandle_t,    // Needs actual type definition
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }
  */

  /*
  // Support for this is not yet implemented as the builtin plugin does not need
  // it
  fn check_remote_datawriter_dispose_instance(
    &self,
    permissions_handle: PermissionsHandle,
    reader: DataReader,                   // Needs actual type definition
    publication_handle: InstanceHandle_t, // Needs actual type definition
    key: DynamicData,                     // Needs actual type definition
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }
  */
}
