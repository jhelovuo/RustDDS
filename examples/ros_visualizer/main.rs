use std::{
  io,
  error::Error,
  time::{Duration, Instant},
};
use std::{thread, time, error};

use log::{debug, error, trace, LevelFilter};
use log4rs::{
  append::console::ConsoleAppender,
  config::{Appender, Root},
  Config,
};

mod display_string;
use crate::display_string::get_topics_view_strings;
use crate::display_string::get_external_node_info_strings;
use crate::display_string::get_local_ros_participant_info_strings;





use rustdds::dds::{
  data_types::{DDSDuration, TopicKind},
  qos::{
    policy::{Deadline, Durability, History, Reliability},
    QosPolicyBuilder,
  },
  statusevents::StatusEvented,
  traits::{Keyed, TopicDescription},
  DomainParticipant,
};
use serde::{Deserialize, Serialize};
use clap::{App, Arg, ArgMatches}; // command line argument processing
//use mio::{Events, Poll, PollOpt, Ready, Token}; // polling
//use mio_extras::channel; // pollable channel
//use rand::prelude::*;


use rustdds::dds::traits::RTPSEntity;
use rustdds::ros2::RosParticipant;
use rustdds::ros2::NodeOptions;
use rustdds::ros2::RosNode;
//use rustdds::ros2::builtin_datatypes::NodeInfo;
//use rustdds::dds::qos::QosPolicies;
//use rustdds::serialization::CDRSerializerAdapter;

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen, EnterAlternateScreen},
};

use tui::{
  backend::{Backend, CrosstermBackend},
  layout::{Constraint, Direction, Layout},
  style::{Color, Modifier, Style},
  text::{Span, Spans},
  widgets::{Block, Borders, Tabs, List, ListItem,ListState},
  Frame, Terminal,
};


struct StatefulList<T> {
  state: ListState,
  items: Vec<T>,
}

impl<T> StatefulList<T> {

  fn push (&mut self, item : T){
    self.items.push(item);
  } 

  fn with_items(items: Vec<T>) -> StatefulList<T> {
      StatefulList {
          state: ListState::default(),
          items,
      }
  }

  fn next(&mut self) {
    if self.items.len() == 0{
      return;
    }
      let i = match self.state.selected() {
          Some(i) => {
              if i >= self.items.len() - 1 {
                  0
              } else {
                  i + 1
              }
          }
          None => 0,
      };
      self.state.select(Some(i));
  }

  fn previous(&mut self) {
      if self.items.len() == 0{
        return;
      }
      let i = match self.state.selected() {
          Some(i) => {
              if i == 0 {
                  self.items.len() - 1
              } else {
                  i - 1
              }
          }
          None => 0,
      };
      self.state.select(Some(i));
  }

  fn unselect(&mut self) {
      self.state.select(None);
  }
}


struct VisualizatorApp<'a> {
  pub titles: Vec<&'a str>,
  pub index: usize,

  pub domain_participant : DomainParticipant,
  pub ros_participant : RosParticipant,
  pub ros_node : RosNode,

  pub topic_list_items : StatefulList<(ListItem<'a>)>
}

impl<'a> VisualizatorApp<'a> {
  fn new( domain_participant: DomainParticipant, ros_participant : RosParticipant, ros_node : RosNode) -> VisualizatorApp<'a> {
    VisualizatorApp {
          titles: vec!["Participants", "Topics",],
          index: 0,
          domain_participant,
          ros_participant,
          ros_node,
          topic_list_items : StatefulList::with_items(vec!())
      }
  }

  pub fn next(&mut self) {
      self.index = (self.index + 1) % self.titles.len();
  }

  pub fn previous(&mut self) {
      if self.index > 0 {
          self.index -= 1;
      } else {
          self.index = self.titles.len() - 1;
      }
  }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: VisualizatorApp, tick_rate: Duration) -> io::Result<()> {
  let mut last_tick = Instant::now();
  loop {
      app.ros_participant.clear();
      app.ros_participant.handle_node_read();


      terminal.draw(|f| ui(f, &mut app))?;


      let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Right => app.next(),
                    KeyCode::Left => app.previous(),
                    KeyCode::Up => app.topic_list_items.previous(),
                    KeyCode::Down => app.topic_list_items.next(),
                    _ => {}
                }
            }
        }
        
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }


      /*
      if let Event::Key(key) = event::read()? {
          match key.code {
              KeyCode::Char('q') => return Ok(()),
              KeyCode::Right => app.next(),
              KeyCode::Left => app.previous(),
              _ => {}
          }
      }
      */
  }
}


fn ui<B: Backend>(f: &mut Frame<B>, app: &mut VisualizatorApp) {
  let size = f.size();
  let chunks = Layout::default()
      .direction(Direction::Vertical)
      .margin(5)
      .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
      .split(size);

  let block = Block::default().style(Style::default().bg(Color::Black).fg(Color::White));
  f.render_widget(block, size);
  let titles = app
      .titles
      .iter()
      .map(|t| {
          let (first, rest) = t.split_at(1);
          Spans::from(vec![
              Span::styled(first, Style::default().fg(Color::Yellow)),
              Span::styled(rest, Style::default().fg(Color::Green)),
          ])
      })
      .collect();
  let tabs = Tabs::new(titles)
      .block(Block::default().borders(Borders::ALL).title("Tabs"))
      .select(app.index)
      .style(Style::default().fg(Color::Cyan))
      .highlight_style(
          Style::default()
              .add_modifier(Modifier::BOLD)
              .bg(Color::Black),
      );
  f.render_widget(tabs, chunks[0]);

  

  let topic_strings = get_topics_view_strings(&app.ros_participant);

  //DOTO CLEAN LIST ?
  let previous_state = app.topic_list_items.state.clone();
  app.topic_list_items = StatefulList::with_items(vec!());
  for string in topic_strings{
    let new_item = ListItem::new(string.clone());
    if app.topic_list_items.items.iter().any(|x| x == &new_item){
      //ALREADY IN List
    }else{
      app.topic_list_items.push(ListItem::new(string));
    }
  }
  app.topic_list_items.state = previous_state;

  let list_of_topics = List::new(app.topic_list_items.items.clone())
    .block(Block::default().title("Topics").borders(Borders::ALL))
    .style(Style::default().fg(Color::White))
    .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
    .highlight_symbol(">>");

  let participant_strings = get_external_node_info_strings(&app.ros_participant);
  let local_participant_strings = get_local_ros_participant_info_strings(&app.ros_participant);
  let mut participant_list_items = vec!();
  for string in participant_strings{
    participant_list_items.push(ListItem::new(string));
  }
  for string in local_participant_strings{
    participant_list_items.push(ListItem::new(string));
  }

  let list_of_participants = List::new(participant_list_items)
  .block(Block::default().title("Participants").borders(Borders::ALL))
  .style(Style::default().fg(Color::White))
  .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
  .highlight_symbol(">>");

  match app.index{
    0 => {f.render_widget(list_of_participants, chunks[1]);},
    1 => {f.render_stateful_widget(list_of_topics, chunks[1],&mut app.topic_list_items.state);}
    2 => {},
    3 => {},
    _ => unreachable!(),
  };

}


fn configure_logging() {
  // initialize logging, preferably from config file
  log4rs::init_file(
    "examples/ros_visualizer/logging-config.yaml",
    log4rs::config::Deserializers::default(),
  )
  .unwrap_or_else(|e| {
    match e.downcast_ref::<io::Error>() {
      // Config file did not work. If it is a simple "No such file or directory", then
      // substitute some default config.
      Some(os_err) if os_err.kind() == io::ErrorKind::NotFound => {
        println!("No config file found in current working directory.");
        let stdout = ConsoleAppender::builder().build();
        let conf = Config::builder()
          .appender(Appender::builder().build("stdout", Box::new(stdout)))
          .build(Root::builder().appender("stdout").build(LevelFilter::Error))
          .unwrap();
        log4rs::init_config(conf).unwrap();
      }
      // Give up.
      other_error => panic!("Config problem: {:?}", other_error),
    }
  });
}



fn main()  -> Result<(), Box<dyn Error>>  {
  configure_logging();
  enable_raw_mode().unwrap();
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend).unwrap();
    // create app and run it
  let domain_id = 0u16;
  let domain_participant = DomainParticipant::new(domain_id)
    .unwrap_or_else(|e| panic!("DomainParticipant construction failed: {:?}", e));
    
  let mut ros_participant = RosParticipant::new().unwrap();
  let ros_node = ros_participant.new_ros_node("local_ node", "/ros2_demo", NodeOptions::new(true)).unwrap();
  
  let mut visualizor_app = VisualizatorApp::new(domain_participant,ros_participant,ros_node);

  let tick_rate = Duration::from_millis(250);
  let res = run_app(&mut terminal, visualizor_app,tick_rate);

  disable_raw_mode()?;
  execute!(
      terminal.backend_mut(),
      LeaveAlternateScreen,
      DisableMouseCapture
  )?;
  terminal.show_cursor()?;  

  return Ok(());

}