use std::time::Duration;

use crate::{discovery::builtin_endpoint::BuiltinEndpointSet, structure::guid::EntityId};

pub const PREEMPTIVE_ACKNACK_PERIOD: Duration = Duration::from_secs(5);

// RTPS spec Section 8.4.7.1.1  "Default Timing-Related Values"
pub const NACK_RESPONSE_DELAY: Duration = Duration::from_millis(200);
pub const NACK_SUPPRESSION_DURATION: Duration = Duration::from_millis(0);

// Helper list for initializing remote standard (non-secure) built-in readers
pub const STANDARD_BUILTIN_READERS_INIT_LIST: &[(EntityId, EntityId, u32)] = &[
  (
    EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER, // SPDP
    EntityId::SPDP_BUILTIN_PARTICIPANT_READER,
    BuiltinEndpointSet::PARTICIPANT_DETECTOR,
  ),
  (
    EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_WRITER, // SEDP ...
    EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_READER,
    BuiltinEndpointSet::SUBSCRIPTIONS_DETECTOR,
  ),
  (
    EntityId::SEDP_BUILTIN_PUBLICATIONS_WRITER,
    EntityId::SEDP_BUILTIN_PUBLICATIONS_READER,
    BuiltinEndpointSet::PUBLICATIONS_DETECTOR,
  ),
  (
    EntityId::SEDP_BUILTIN_TOPIC_WRITER,
    EntityId::SEDP_BUILTIN_TOPIC_READER,
    BuiltinEndpointSet::TOPICS_DETECTOR,
  ),
  (
    EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
    EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_READER,
    BuiltinEndpointSet::PARTICIPANT_MESSAGE_DATA_READER,
  ),
];

// Helper list for initializing remote standard (non-secure) built-in writers
pub const STANDARD_BUILTIN_WRITERS_INIT_LIST: &[(EntityId, EntityId, u32)] = &[
  (
    EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER, // SPDP
    EntityId::SPDP_BUILTIN_PARTICIPANT_READER,
    BuiltinEndpointSet::PARTICIPANT_ANNOUNCER,
  ),
  (
    EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_WRITER, // SEDP ...
    EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_READER,
    BuiltinEndpointSet::PUBLICATIONS_ANNOUNCER,
  ),
  (
    EntityId::SEDP_BUILTIN_PUBLICATIONS_WRITER,
    EntityId::SEDP_BUILTIN_PUBLICATIONS_READER,
    BuiltinEndpointSet::PUBLICATIONS_ANNOUNCER,
  ),
  (
    EntityId::SEDP_BUILTIN_TOPIC_WRITER,
    EntityId::SEDP_BUILTIN_TOPIC_READER,
    BuiltinEndpointSet::TOPICS_ANNOUNCER,
  ),
  (
    EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
    EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_READER,
    BuiltinEndpointSet::PARTICIPANT_MESSAGE_DATA_WRITER,
  ),
];

// Helper list for initializing the authentication topic built-in reader
pub const AUTHENTICATION_BUILTIN_READERS_INIT_LIST: &[(EntityId, EntityId, u32)] = &[(
  EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_WRITER,
  EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_READER,
  BuiltinEndpointSet::PARTICIPANT_STATELESS_MESSAGE_READER,
)];

// Helper list for initializing the authentication topic built-in writer
pub const AUTHENTICATION_BUILTIN_WRITERS_INIT_LIST: &[(EntityId, EntityId, u32)] = &[(
  EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_WRITER,
  EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_READER,
  BuiltinEndpointSet::PARTICIPANT_STATELESS_MESSAGE_WRITER,
)];

// Helper list for initializing remote secure built-in readers
pub const SECURE_BUILTIN_READERS_INIT_LIST: &[(EntityId, EntityId, u32)] = &[
  (
    EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_WRITER, // SPDP
    EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_READER,
    BuiltinEndpointSet::PARTICIPANT_SECURE_READER,
  ),
  (
    EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_WRITER, // SEDP ...
    EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_READER,
    BuiltinEndpointSet::PUBLICATIONS_SECURE_READER,
  ),
  (
    EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_WRITER,
    EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_READER,
    BuiltinEndpointSet::SUBSCRIPTIONS_SECURE_READER,
  ),
  (
    EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_SECURE_WRITER,
    EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_SECURE_READER,
    BuiltinEndpointSet::PARTICIPANT_MESSAGE_SECURE_READER,
  ),
  (
    EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_WRITER,
    EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_READER,
    BuiltinEndpointSet::PARTICIPANT_VOLATILE_MESSAGE_SECURE_READER,
  ),
];

// Helper list for initializing remote secure built-in writers
pub const SECURE_BUILTIN_WRITERS_INIT_LIST: &[(EntityId, EntityId, u32)] = &[
  (
    EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_WRITER, // SPDP
    EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_READER,
    BuiltinEndpointSet::PARTICIPANT_SECURE_WRITER,
  ),
  (
    EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_WRITER, // SEDP ...
    EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_READER,
    BuiltinEndpointSet::PUBLICATIONS_SECURE_WRITER,
  ),
  (
    EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_WRITER,
    EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_READER,
    BuiltinEndpointSet::SUBSCRIPTIONS_SECURE_WRITER,
  ),
  (
    EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_SECURE_WRITER,
    EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_SECURE_READER,
    BuiltinEndpointSet::PARTICIPANT_MESSAGE_SECURE_WRITER,
  ),
  (
    EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_WRITER,
    EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_READER,
    BuiltinEndpointSet::PARTICIPANT_VOLATILE_MESSAGE_SECURE_WRITER,
  ),
];

// EntityIds for built-in readers with secured communication
// See the definition of “Builtin Secure Endpoints” in the Security spec
// This list is used for detecting if a built-in reader needs to be secure.
// TODO: STANDARD_BUILTIN_READERS_INIT_LIST already contains these
// EntityIds. Could we use that list directly and get rid of this one?
pub const SECURE_BUILTIN_READER_ENTITY_IDS: &[EntityId] = &[
  EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_READER,
  EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_READER,
  EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_READER,
  EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_SECURE_READER,
  EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_READER,
];

// EntityIds for built-in writers with secured communication
// This list is used for detecting if a built-in writer needs to be secure.
// TODO: STANDARD_BUILTIN_WRITERS_INIT_LIST already contains these
// EntityIds. Could we use that list directly and get rid of this one?
pub const SECURE_BUILTIN_WRITER_ENTITY_IDS: &[EntityId] = &[
  EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_WRITER,
  EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_WRITER,
  EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_WRITER,
  EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_SECURE_WRITER,
  EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_WRITER,
];
