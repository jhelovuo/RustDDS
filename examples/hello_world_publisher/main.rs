//! Re-implementation of helloworld / publisher example in CycloneDDS
use std::time::Duration;

use log::error;
use rustdds::{
  policy::Reliability, DataWriterStatus, DomainParticipantBuilder, Keyed, QosPolicyBuilder,
  StatusEvented, TopicKind,
};
use serde::{Deserialize, Serialize};
use smol::Timer;
use futures::{FutureExt, StreamExt, TryFutureExt};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct HelloWorldData {
  pub user_id: i32,
  pub message: String,
}

impl Keyed for HelloWorldData {
  type K = i32;
  fn key(&self) -> Self::K {
    self.user_id
  }
}

fn main() {
  let domain_participant = DomainParticipantBuilder::new(0)
    .build()
    .unwrap_or_else(|e| panic!("DomainParticipant construction failed: {e:?}"));

  let qos = QosPolicyBuilder::new()
    .reliability(Reliability::Reliable {
      max_blocking_time: rustdds::Duration::from_secs(1),
    })
    .build();

  let topic = domain_participant
    .create_topic(
      // We can internally call the Rust type "HelloWorldData" whatever we want,
      // but these strings must match whatever our counterparts expect
      // to see over RTPS.
      "HelloWorldData_Msg".to_string(),  // topic name
      "HelloWorldData::Msg".to_string(), // type name
      &qos,
      TopicKind::WithKey,
    )
    .unwrap_or_else(|e| panic!("create_topic failed: {e:?}"));

  let publisher = domain_participant.create_publisher(&qos).unwrap();
  let writer = publisher
    .create_datawriter_cdr::<HelloWorldData>(&topic, None) // None = get qos policy from publisher
    .unwrap();

  let hello_message = HelloWorldData {
    user_id: 42,
    message: "Hello, Rust!".to_string(),
  };

  // set up async executor to run concurrent tasks
  smol::block_on(async {
    let mut datawriter_event_stream = writer.as_async_status_stream();
    let (write_trigger_sender, write_trigger_receiver) = smol::channel::bounded(1);
    let mut match_timeout_timer = futures::FutureExt::fuse(Timer::after(Duration::from_secs(10)));

    println!("Ready to say hello");
    loop {
      futures::select! {
        _ = match_timeout_timer => {
          println!("Timeout waiting for subscriber at appear.");
          break
        }
        _ = write_trigger_receiver.recv().fuse() => {
          println!("Sending hello");
          writer.async_write(hello_message.clone(), None)
            .unwrap_or_else(|e| error!("DataWriter async_write failed: {e:?}"))
            .await;
          // wait for 1 sec for transfer to complete before exiting.
          Timer::after(Duration::from_secs(1)).await;
          break
        }
        e = datawriter_event_stream.select_next_some() => {
          match e {
            // If we get a matching subscription, trigger the send
            DataWriterStatus::PublicationMatched{..} => {
              println!("Matched with hello subscriber");
              // Wait for a while so that subscriber also recognizes us.
              // There is no two- or three-way handshake in pub/sub matching,
              // so we cannot know if the other side is immediately ready.
              Timer::after(Duration::from_secs(1)).await;
              write_trigger_sender.send(()).await.unwrap();
            }
            _ =>
              println!("DataWriter event: {e:?}"),
          }
        }
      } // select!
    } // loop

    println!("Bye, World!");
  });
}
