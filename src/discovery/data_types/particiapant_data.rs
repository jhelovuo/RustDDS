use crate::discovery::participant_proxy::ParticipantProxyAttributes;
use std::time::Duration;

// specializes the ParticipantProxy.
// The SPDPdiscoveredParticipantData defines the data exchanged as part of the SPDP
pub struct SPDSdiscoveredParticipantData {
  ///How long a Participant should be considered alive every
  /// time an announcement is received from the Participant.
  /// If a Participant fails to send another announcement
  /// within this time period, the Participant can be
  /// considered gone. In that case, any resources associated
  /// to the Participant and its Endpoints can be freed.
  lease_duration: Duration,
}

impl ParticipantProxy for SPDSdiscoveredParticipantData {
  fn as_participant_proxy(
    &self,
  ) -> &crate::discovery::participant_proxy::ParticipantProxyAttributes {
    todo!()
  }
}
