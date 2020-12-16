extern crate rustdds;
extern crate serde;
extern crate mio;
extern crate mio_extras;
extern crate byteorder;
extern crate termion;

use rustdds::{
  serialization::{CDRSerializerAdapter, CDRDeserializerAdapter},
  dds::{
    DomainParticipant, qos::QosPolicies, data_types::ReadCondition, With_Key_DataReader,
    data_types::TopicKind,
  },
  dds::qos::policy::Reliability,
  dds::data_types::DDSDuration,
  dds::qos::policy::History,
  dds::qos::policy::Ownership,
  dds::qos::policy::Durability,
  dds::qos::policy::Liveliness,
  dds::qos::policy::DestinationOrder,
  dds::qos::policy::ResourceLimits,
  dds::qos::policy::Deadline,
  dds::qos::policy::LatencyBudget,
  dds::qos::policy::Presentation,
  dds::qos::policy::PresentationAccessScope,
  dds::qos::policy::Lifespan,
  dds::traits::Keyed,
};
use std::{
  sync::{
    atomic::{Ordering, AtomicBool},
    Arc,
  },
  time::Duration as StdDuration,
};
use mio::{Poll, Token, Ready, PollOpt, Events};
use mio_extras::{timer::Timer};
use shapes::Square;
use std::io::{Write, Read};
use termion::raw::IntoRawMode;
use byteorder::LittleEndian;

mod shapes;

fn main() {
  env_logger::init();

  let domain_id = std::env::args().nth(1).unwrap_or(String::from("0"));
  let domain_id = domain_id.parse::<u16>().unwrap();

  let running = Arc::new(AtomicBool::new(true));
  let r = running.clone();
  ctrlc::set_handler(move || {
    r.store(false, Ordering::SeqCst);
  })
  .expect("Error setting Ctrl-C handler");
  println!("Waiting for Ctrl-C...");

  println!("Use cursor keys to control Triangle.");
  println!("Set up another DDS publisher for Square topic.");

  let loop_thread = std::thread::spawn(move || event_loop(running, domain_id));
  loop_thread.join().expect("The loop thread has panicked");
}

// milliseconds
const KEYBOARD_CHECK_TIMEOUT: u64 = 50;

// declaring event loop tokens for better readability
const STOP_EVENT_LOOP_TOKEN: Token = Token(1000);
const SQUARE_READER_TOKEN: Token = Token(1001);
const KEYBOARD_CHECK_TOKEN: Token = Token(1002);

fn event_loop(running_flag: Arc<AtomicBool>, domain_id: u16) {
  let poll = Poll::new().unwrap();

  // adjust domain_id or participant_id if necessary to interoperability
  let domain_participant = DomainParticipant::new(domain_id);
  //println!("Have DP");
  let pub_qos = QosPolicies::builder()
    .reliability(Reliability::BestEffort)
    .history(History::KeepLast { depth: 1 })
    .ownership(Ownership::Shared)
    .durability(Durability::Volatile)
    .liveliness(Liveliness::Automatic {
      lease_duration: DDSDuration::DURATION_INFINITE,
    })
    .destination_order(DestinationOrder::ByReceptionTimestamp)
    .resource_limits(ResourceLimits {
      max_instances: std::i32::MAX,
      max_samples: std::i32::MAX,
      max_samples_per_instance: std::i32::MAX,
    })
    .deadline(Deadline(DDSDuration::DURATION_INFINITE))
    .latency_budget(LatencyBudget {
      duration: DDSDuration::DURATION_ZERO,
    })
    .presentation(Presentation {
      access_scope: PresentationAccessScope::Instance,
      coherent_access: false,
      ordered_access: false,
    })
    .lifespan(Lifespan {
      duration: DDSDuration::DURATION_INFINITE,
    })
    .build();

  //println!("Create topics");

  // declare topics, subscriber, publisher, readers and writers
  let square_topic = domain_participant
    .create_topic("Square", "ShapeType", &pub_qos, TopicKind::WithKey)
    .unwrap();
  //println!("Square");
  let triangle_topic = domain_participant
    .create_topic("Triangle", "ShapeType", &pub_qos, TopicKind::WithKey)
    .unwrap();
  //println!("Triangle. Have topics. Create subscriber.");

  let shapes_sub = domain_participant
    .create_subscriber(&QosPolicies::builder().build())
    .unwrap();

  //println!("Have subscriber");

  // reader needs to be mutable if you want to read/take something from it
  let mut square_reader = shapes_sub
    .create_datareader::<Square, CDRDeserializerAdapter<Square>>(square_topic, None, None)
    .unwrap();

  let shapes_pub = domain_participant.create_publisher(&pub_qos).unwrap();
  let triangle_writer = shapes_pub
    .create_datawriter::<Square, CDRSerializerAdapter<Square, LittleEndian>>(
      None,
      triangle_topic,
      None,
    )
    .unwrap();

  //println!("Have reader and writer");

  // register readers and possible timers
  // poll
  //   .register(
  //     &stop_receiver,
  //     STOP_EVENT_LOOP_TOKEN,
  //     Ready::readable(),
  //     PollOpt::edge(),
  //   )
  //   .unwrap();
  poll
    .register(
      &square_reader,
      SQUARE_READER_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();
  
  std::thread::sleep(std::time::Duration::from_millis(2000));

  let stdout_org = std::io::stdout();
  let mut areader = termion::async_stdin().bytes();
  {
    let mut stdout = stdout_org.lock().into_raw_mode().unwrap();
    write!(
      stdout,
      "{}{} ",
      termion::clear::All,
      termion::cursor::Goto(1, 1)
    )
    .unwrap();
    stdout.flush().unwrap();
  }

  let mut input_timer = Timer::default();
  input_timer.set_timeout(StdDuration::from_millis(KEYBOARD_CHECK_TIMEOUT), ());
  poll
    .register(
      &input_timer,
      KEYBOARD_CHECK_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  let mut row: u16 = 0;
  let mut square = Square::new(String::from("BLUE"), 0, 0, 30);

  loop {
    // check for Ctrl-C
    if  ! running_flag.load(Ordering::SeqCst) { return }

    {
      if row > 60 {
        let mut stdout = stdout_org.lock().into_raw_mode().unwrap();
        row = 1;
        write!(
          stdout,
          "{}{}",
          termion::clear::All,
          termion::cursor::Goto(1, row)
        )
        .unwrap();
        stdout.flush().unwrap();
      }
      // {
      //   let mut stdout = stdout_org.lock().into_raw_mode().unwrap();
      //   write!(stdout, "Polling...").unwrap(); stdout.flush().unwrap();
      // }
      let mut events = Events::with_capacity(10);
      poll.poll(&mut events, None).unwrap();

      for event in events.iter() {
        let mut stdout = stdout_org.lock().into_raw_mode().unwrap();

        if event.token() == STOP_EVENT_LOOP_TOKEN {
          return;
        } else if event.token() == SQUARE_READER_TOKEN {
          let squares = fetch_squares(&mut square_reader);
          for square in squares.iter() {
            write!(stdout, "{}", termion::cursor::Goto(1, row)).unwrap();
            write!(stdout, "Item: {:?} received", square).unwrap();
            stdout.flush().unwrap();
            row += 1;
          }
        } else if event.token() == KEYBOARD_CHECK_TOKEN {
          let mut square_moved = false;
          let mut dispose_square = false;

          while let Some(c) = areader.next() {
            write!(stdout, "{}", termion::cursor::Goto(1, row))
              .unwrap();

            let c = match c {
              Ok(c) => c,
              _ => {
                continue;
              }
            };
            match c {
              113 => { // "q"
                print!("Exiting main loop");
                stdout.flush().unwrap();
                return
              }
              65 => { // cursor down
                print!("Y-");
                square.yadd(-1);
                square_moved = true;
              }
              66 => { // cursor up
                print!("Y+");
                square.yadd(1);
                square_moved = true;
              }
              67 => { // cursor right
                print!("X+");
                square.xadd(1);
                square_moved = true;
              }
              68 => {
                print!("X-");
                square.xadd(-1);
                square_moved = true;
              }
              100 => { // "d"
                print!("dispose!");
                dispose_square = true;
              }
              _ => {
                continue;
              }
            };
          }
          stdout.flush().unwrap();
          if square_moved {
            match triangle_writer.write(square.clone(), None) {
              Ok(_) => (),
              Err(e) => println!("Failed to write new square. {:?}", e),
            };
          }

          if dispose_square {
            println!("Disposing square");
            match triangle_writer.dispose(square.get_key(), None) {
              Ok(_) => (),
              Err(e) => println!("Failed to dispose square. {:?}", e),
            }
          }

          input_timer.set_timeout(StdDuration::from_millis(KEYBOARD_CHECK_TIMEOUT), ());
        }
      }
    }
  }
}

fn fetch_squares(
  reader: &mut With_Key_DataReader<Square, CDRDeserializerAdapter<Square>>,
) -> Vec<Square> {
  match reader.take(10, ReadCondition::any()) {
    Ok(ds) => ds.into_iter().filter_map(|p| p.into_value().ok()).collect(),
    Err(_) => {
      println!("Failed to read squares");
      vec![]
    }
  }
}
