//! Re-implementation of helloworld / subscriber example in CycloneDDS

use rustdds::{
  policy::Reliability, with_key::Sample, DataReaderStatus, DomainParticipantBuilder, Keyed,
  QosPolicyBuilder, TopicKind,
};
use serde::{Deserialize, Serialize};
use futures::{FutureExt, StreamExt};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct HelloWorldData {
  pub user_id: i32,
  pub message: String,
}

// This is made Keyed after the example in CycloneDDS.
// There is no use for the key in this example.
impl Keyed for HelloWorldData {
  type K = i32;

  fn key(&self) -> Self::K {
    self.user_id
  }
}

fn main() {
  // Set Ctrl-C handler
  let (stop_sender, stop_receiver) = smol::channel::bounded(1);
  ctrlc::set_handler(move || {
    stop_sender.send_blocking(()).unwrap_or(());
    // ignore errors, as we are quitting anyway
  })
  .expect("Error setting Ctrl-C handler");
  println!("Press Ctrl-C to quit.");

  let domain_participant = DomainParticipantBuilder::new(0)
    .build()
    .unwrap_or_else(|e| panic!("DomainParticipant construction failed: {e:?}"));

  let qos = QosPolicyBuilder::new()
    .reliability(Reliability::BestEffort)
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

  let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  let data_reader = subscriber
    .create_datareader_cdr::<HelloWorldData>(&topic, None) // None = get qos policy from publisher
    .unwrap();

  // set up async executor to run concurrent tasks
  smol::block_on(async {
    let mut sample_stream = data_reader.async_sample_stream();
    let mut event_stream = sample_stream.async_event_stream();

    println!("Waiting for hello messages.");
    loop {
      futures::select! {
        _ = stop_receiver.recv().fuse() =>
          break,

        result = sample_stream.select_next_some() => {
          match result {
            Ok(s) => match s.into_value() {
              Sample::Value(hello_msg) =>
                println!("Received: {hello_msg:?}"),
              Sample::Dispose(key) =>
                println!("Disposed hello with key={key}"),
            }
            Err(e) =>
              println!("Oh no, DDS read error: {e:?}"),
          }
        }

        e = event_stream.select_next_some() => {
          match e {
            DataReaderStatus::SubscriptionMatched{ writer, current,..} => {
              if current.count_change() > 0 {
                println!("Matched with hello publisher {writer:?}");
              } else {
                println!("Lost hello publisher {writer:?}");
              }
            }
            _ =>
              println!("DataReader event: {e:?}"),
          }
        }
      } // select!
    } // loop

    println!("\nBye, World!");
  });
}
