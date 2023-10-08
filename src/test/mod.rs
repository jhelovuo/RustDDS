#![cfg(test)]

pub(crate) mod random_data;
pub(crate) mod shape_type;
pub(crate) mod test_data;
pub(crate) mod test_properties;

use anyhow::Result;
use crate::{
    policy::{Durability, History, Reliability},
    DomainParticipant, QosPolicyBuilder, Timestamp, TopicKind,
};
use std::{thread, time::Duration};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct TestType;

#[test]
fn timestamp() -> Result<()> {
    let participant = DomainParticipant::new(0)?;
    let qos = QosPolicyBuilder::new()
        .history(History::KeepAll)
        .reliability(Reliability::Reliable {
            max_blocking_time: Duration::from_secs(0).into(),
        })
        .durability(Durability::TransientLocal)
        .build();
    let writer = participant
        .create_publisher(&qos)?
        .create_datawriter_no_key_cdr::<TestType>(
            &participant.create_topic(
                "test".to_string(),
                "TestType".to_string(),
                &qos,
                TopicKind::NoKey,
            )?,
            None,
        )?;
    let participant2 = DomainParticipant::new(0)?;
    let mut reader = participant2
        .create_subscriber(&qos)?
        .create_datareader_no_key_cdr::<TestType>(
            &participant.create_topic(
                "test".to_string(),
                "TestType".to_string(),
                &qos,
                TopicKind::NoKey,
            )?,
            None,
        )?;
    let timestamp = Timestamp::now();
    writer.write(TestType, Some(timestamp))?;
    thread::sleep(Duration::from_secs(3));
    loop {
        if let Ok(Some(sample)) = reader.take_next_sample() {
            assert_eq!(timestamp, sample.sample_info().source_timestamp().unwrap());
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}