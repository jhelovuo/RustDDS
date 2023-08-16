use crate::{
  discovery::sedp_messages::TopicBuiltinTopicData,
  security::{access_control::*, *},
};
use super::AccessControlBuiltin;

impl RemoteEntityAccessControl for AccessControlBuiltin {
  // Currently only mocked
  fn check_remote_participant(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    participant_data: &ParticipantBuiltinTopicDataSecure,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  // Currently only mocked
  fn check_remote_datawriter(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    publication_data: &PublicationBuiltinTopicDataSecure,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  // Currently only mocked
  fn check_remote_datareader(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    subscription_data: &SubscriptionBuiltinTopicDataSecure,
    relay_only: &mut bool,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  // Currently only mocked
  fn check_remote_topic(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_data: &TopicBuiltinTopicData,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  fn check_local_datawriter_match(
    &self,
    writer_permissions_handle: PermissionsHandle,
    reader_permissions_handle: PermissionsHandle,
    publication_data: &PublicationBuiltinTopicDataSecure,
    subscription_data: &SubscriptionBuiltinTopicDataSecure,
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }

  fn check_local_datareader_match(
    &self,
    reader_permissions_handle: PermissionsHandle,
    writer_permissions_handle: PermissionsHandle,
    subscription_data: &SubscriptionBuiltinTopicDataSecure,
    publication_data: &PublicationBuiltinTopicDataSecure,
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }

  fn check_remote_datawriter_register_instance(
    &self,
    permissions_handle: PermissionsHandle,
    reader_todo: (),
    publication_handle_todo: (),
    key_todo: (),
    instance_handle_todo: (),
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }

  fn check_remote_datawriter_dispose_instance(
    &self,
    permissions_handle: PermissionsHandle,
    reader_todo: (),
    publication_handle_todo: (),
    key_todo: (),
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }
}
