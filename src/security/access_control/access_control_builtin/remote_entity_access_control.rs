use std::ops::Not;

use crate::{
  discovery::{
    sedp_messages::TopicBuiltinTopicData, DiscoveredReaderData, DiscoveredWriterData,
    PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
  },
  security::{access_control::*, *},
  security_error,
};
use super::{
  domain_governance_document::TopicRule, domain_participant_permissions_document::Action,
  types::Entity, AccessControlBuiltin,
};

impl RemoteEntityAccessControl for AccessControlBuiltin {
  fn check_remote_datawriter(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    publication_data: &PublicationBuiltinTopicDataSecure,
  ) -> SecurityResult<()> {
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
  ) -> SecurityResult<bool> {
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

    // TODO: remove after testing
    if true {
      return Ok(false);
    }

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

    (requested_access_is_unprotected || participant_has_read_access)
      .then_some(false)
      // Check for relay only access
      .or_else(|| {
        bool::from(grant.check_action(Action::Relay, domain_id, topic_name, partitions, data_tags))
          .then_some(true)
      })
      .ok_or_else(|| security_error!("The participant has no read nor relay access to the topic."))
  }

  fn check_remote_topic(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_data: &TopicBuiltinTopicData,
  ) -> SecurityResult<()> {
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

  fn check_remote_datawriter_register_instance(
    &self,
    _permissions_handle: PermissionsHandle,
    _reader_todo: (),
    _publication_handle_todo: (),
    _key_todo: (),
    _instance_handle_todo: (),
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }

  fn check_remote_datawriter_dispose_instance(
    &self,
    _permissions_handle: PermissionsHandle,
    _reader_todo: (),
    _publication_handle_todo: (),
    _key_todo: (),
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }
}
