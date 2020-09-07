extern crate atosdds;
extern crate serde;
extern crate mio;
extern crate mio_extras;
extern crate byteorder;
extern crate termion;

use atosdds::{
  serialization::{
    cdrSerializer::CDR_serializer_adapter, cdrDeserializer::CDR_deserializer_adapter,
  },
  dds::{
    typedesc::TypeDesc, participant::DomainParticipant, qos::QosPolicies, datareader::DataReader,
    readcondition::ReadCondition,
  },
};
use std::{
  sync::{
    atomic::{Ordering, AtomicBool},
    Arc,
  },
  time::Duration,
};
use mio::{Poll, Token, Ready, PollOpt, Events};
use mio_extras::{timer::Timer, channel as mio_channel};
use shapes::Square;
use std::io::{Write, Read};
use termion::raw::IntoRawMode;
use byteorder::LittleEndian;

mod shapes;

fn main() {
  let domain_id = std::env::args().nth(1).unwrap_or(String::from("0"));
  let domain_id = domain_id.parse::<u16>().unwrap();
  let participant_id = std::env::args().nth(2).unwrap_or(String::from("0"));
  let participant_id = participant_id.parse::<u16>().unwrap();

  let (stop_channel_sender, stop_channel_receiver) = mio_channel::channel();
  let _jhandle =
    std::thread::spawn(move || event_loop(stop_channel_receiver, domain_id, participant_id));

  stop_control(stop_channel_sender);
}

// milliseconds
const KEYBOARD_CHECK_TIMEOUT: u64 = 50;

// declaring event loop tokens for better readability
const STOP_EVENT_LOOP_TOKEN: Token = Token(1000);
const SQUARE_READER_TOKEN: Token = Token(1001);
const KEYBOARD_CHECK_TOKEN: Token = Token(1002);

fn event_loop(stop_receiver: mio_channel::Receiver<()>, domain_id: u16, participant_id: u16) {
  let poll = Poll::new().unwrap();

  // adjust domain_id or participant_id if necessary to interoperability
  let domain_participant = DomainParticipant::new(domain_id, participant_id);

  // declare topics, subscriber, publisher, readers and writers
  let square_topic = domain_participant
    .create_topic(
      "Shapes",
      TypeDesc::new(String::from("Square")),
      &QosPolicies::qos_none(),
    )
    .unwrap();
  let square_sub = domain_participant
    .create_subscriber(&QosPolicies::qos_none())
    .unwrap();
  // reader needs to be mutable if you want to read/take something from it
  let mut square_reader = square_sub
    .create_datareader::<Square, CDR_deserializer_adapter<Square>>(
      None,
      &square_topic,
      &QosPolicies::qos_none(),
    )
    .unwrap();

  let square_pub = domain_participant
    .create_publisher(&QosPolicies::qos_none())
    .unwrap();
  let mut square_writer = square_pub
    .create_datawriter::<Square, CDR_serializer_adapter<Square, LittleEndian>>(
      None,
      &square_topic,
      &QosPolicies::qos_none(),
    )
    .unwrap();

  // register readers and possible timers
  poll
    .register(
      &stop_receiver,
      STOP_EVENT_LOOP_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();
  poll
    .register(
      &square_reader,
      SQUARE_READER_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  // let stdout = std::io::stdout();
  // // let mut stdout = stdout.lock().into_raw_mode().unwrap();
  let mut areader = termion::async_stdin().bytes();

  // write!(
  //   stdout,
  //   "{}{}",
  //   termion::clear::All,
  //   termion::cursor::Goto(1, 1)
  // )
  // .unwrap();

  let mut input_timer = Timer::default();
  input_timer.set_timeout(Duration::from_millis(KEYBOARD_CHECK_TIMEOUT), ());
  poll
    .register(
      &input_timer,
      KEYBOARD_CHECK_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  let mut row: u16 = 0;
  let mut square = Square::new(String::from("PURPLE"), 0, 0, 30);
  loop {
    if row % 60 == 0 {
      // write!(
      //   stdout,
      //   "{}{}",
      //   termion::clear::All,
      //   termion::cursor::Goto(1, row)
      // )
      // .unwrap();
      row = 1;
    }
    let mut events = Events::with_capacity(10);
    poll.poll(&mut events, None).unwrap();

    for event in events.iter() {
      if event.token() == STOP_EVENT_LOOP_TOKEN {
        return;
      } else if event.token() == SQUARE_READER_TOKEN {
        let squares = fetch_squares(&mut square_reader);
        for square in squares.iter() {
          // write!(stdout, "{}", termion::cursor::Goto(1, row)).unwrap();
          println!("Item: {:?} received", square);
          row += 1;
        }
      } else if event.token() == KEYBOARD_CHECK_TOKEN {
        while let Some(c) = areader.next() {
          // write!(stdout, "{}", termion::cursor::Goto(1, row)).unwrap();

          let c = match c {
            Ok(c) => c,
            _ => {
              continue;
            }
          };
          match c {
            113 => return,
            65 => {
              square.yadd(1);
              println!("{:?}", square);
              square_writer.write(square.clone(), None);
              row += 1;
            }
            66 => {
              square.yadd(-1);
              println!("{:?}", square);
              square_writer.write(square.clone(), None);
              row += 1;
            }
            67 => {
              square.xadd(1);
              println!("{:?}", square);
              square_writer.write(square.clone(), None);
              row += 1;
            }
            68 => {
              square.xadd(-1);
              println!("{:?}", square);
              square_writer.write(square.clone(), None);
              row += 1;
            }
            _ => continue,
          };
        }

        input_timer.set_timeout(Duration::from_millis(KEYBOARD_CHECK_TIMEOUT), ());
      }
    }
  }
}

fn fetch_squares(reader: &mut DataReader<Square, CDR_deserializer_adapter<Square>>) -> Vec<Square> {
  match reader.take(100, ReadCondition::not_read()) {
    Ok(ds) => ds
      .iter()
      .map(|p| p.value.as_ref())
      .filter(|p| p.is_ok())
      .map(|p| (*p.unwrap()).clone())
      .collect(),
    Err(_) => {
      println!("Failed to read squares");
      vec![]
    }
  }
}

fn stop_control(stop_sender: mio_channel::Sender<()>) {
  let running = Arc::new(AtomicBool::new(true));
  let r = running.clone();

  ctrlc::set_handler(move || {
    r.store(false, Ordering::SeqCst);
  })
  .expect("Error setting Ctrl-C handler");

  println!("Waiting for Ctrl-C...");
  while running.load(Ordering::SeqCst) {}

  match stop_sender.send(()) {
    Ok(_) => (),
    _ => println!("EventLoop is already finished."),
  };
  println!("Got it! Exiting...");
}
