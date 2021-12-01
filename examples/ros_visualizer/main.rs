use std::{
  io,
  error::Error,
  time::{Duration, Instant},
  collections::{HashMap}
};

use log::{LevelFilter};
use log4rs::{
  append::console::ConsoleAppender,
  config::{Appender, Root},
  Config,
};


mod stateful_list;
use crate::stateful_list::StatefulList;
mod display_string;
use crate::display_string::get_topics_list_view_strings;
use crate::display_string::get_topic_view_strings;
use crate::display_string::get_external_node_info_strings;
use crate::display_string::get_local_ros_participant_info_strings;
use crate::display_string::get_node_list_strings;
mod visualization_helpers;
use crate::visualization_helpers::create_paragraph_from_string_list;
use crate::visualization_helpers::create_layput_row;





use rustdds::dds::{
  //data_types::{DDSDuration, TopicKind},
  //qos::{
    //policy::{Deadline, Durability, History, Reliability},
    //QosPolicyBuilder,
  //},
  //statusevents::StatusEvented,
  //traits::{Keyed, TopicDescription},
  
  DomainParticipant,
};
//use serde::{Deserialize, Serialize};
//use clap::{App, Arg, ArgMatches}; // command line argument processing
//use mio::{Events, Poll, PollOpt, Ready, Token}; // polling
//use mio_extras::channel; // pollable channel
//use rand::prelude::*;


//use rustdds::dds::traits::RTPSEntity;
use rustdds::ros2::RosParticipant;
use rustdds::ros2::NodeOptions;
use rustdds::ros2::RosNode;
use rustdds::ros2::builtin_datatypes::NodeInfo;
use rustdds::ros2::builtin_datatypes::Gid;

use rustdds::dds::data_types::DiscoveredTopicData;


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
  widgets::{Block, Borders, Tabs, List, ListItem},//ListState, Paragraph, Wrap},
  Frame, Terminal,
};


struct VisualizatorApp<'a> {
  pub tab_titles: Vec<&'a str>,
  pub tab_index: usize,

  pub domain_participant : DomainParticipant,
  pub ros_participant : RosParticipant,
  pub ros_node : RosNode,

  pub topic_list_items : Vec<DiscoveredTopicData>,
  pub external_nodes : Vec<NodeInfo>,
  pub local_nodes : Vec<NodeInfo>,
  //viewed datas:
  pub topic_list_display_items : StatefulList<ListItem<'a>>,
  pub external_nodes_display_items : StatefulList<ListItem<'a>>,
  pub local_nodes_display_items : StatefulList<ListItem<'a>>,
}

impl<'a> VisualizatorApp<'a> {
  fn new( domain_participant: DomainParticipant, ros_participant : RosParticipant, ros_node : RosNode) -> VisualizatorApp<'a> {
    VisualizatorApp {
          tab_titles: vec!["Participants", "Local Nodes", "Exteral Nodes",  "Topics",],
          tab_index: 0,
          domain_participant,
          ros_participant,
          ros_node,
          external_nodes : vec!(),
          topic_list_items : vec!(),
          local_nodes : vec!(),
          topic_list_display_items : StatefulList::with_items(vec!()),
          external_nodes_display_items : StatefulList::with_items(vec!()),
          local_nodes_display_items : StatefulList::with_items(vec!()),
      }
  }

  pub fn next_tab(&mut self) {
      self.tab_index = (self.tab_index + 1) % self.tab_titles.len();
  }

  pub fn previous_tab(&mut self) {
      if self.tab_index > 0 {
          self.tab_index -= 1;
      } else {
          self.tab_index = self.tab_titles.len() - 1;
      }
  }

  pub fn set_topic_list_items(&mut self){
    self.topic_list_items = self.ros_participant.discovered_topics().clone();
    let topic_strings = get_topics_list_view_strings(&self.topic_list_items);
    let previous_state = self.topic_list_display_items.state.clone();
    self.topic_list_display_items = StatefulList::with_items(vec!());
    for string in topic_strings{
      self.topic_list_display_items.push(ListItem::new(string));
    }
    self.topic_list_display_items.state = previous_state;
  }

  pub fn set_node_list_items(&mut self) {
    let externals = self.ros_participant.get_all_discovered_external_ros_node_infos();
    let locals = self.ros_participant.get_all_discovered_local_ros_node_infos();

    
    //let extarnal_node_infos = externals.into_iter().map(|(gid,nodes)|nodes).collect::<NodeInfo>();

    self.local_nodes = locals.clone().into_iter().map(|(string,nodes)|nodes).collect();
    let external_node_list_string = get_node_list_strings(&self.external_nodes);
    let local_node_list_string = get_node_list_strings(&self.local_nodes);
    let previous_state = self.external_nodes_display_items.state.clone();
    self.external_nodes_display_items = StatefulList::with_items(vec!());
    
    for string in external_node_list_string {
      self.external_nodes_display_items.push(ListItem::new(string));
    }

    self.local_nodes_display_items = StatefulList::with_items(vec!());
    for string in local_node_list_string {
      self.local_nodes_display_items.push(ListItem::new(string));
    }

  }

}

fn handle_user_input(app: &mut VisualizatorApp, timeout : &Duration) -> bool {
  let mut quit_application = false;
  if crossterm::event::poll(*timeout).unwrap() {
    if let Event::Key(key) = event::read().unwrap() {
        match key.code {
            KeyCode::Char('q') => quit_application = true,
            KeyCode::Right => app.next_tab(),
            KeyCode::Left => app.previous_tab(),
            KeyCode::Up => {
              match app.tab_index {
                0 => {},
                1 => {},
                2 => {},
                3 => {app.topic_list_display_items.previous()},
                _ => {},
              }

            }, 
            KeyCode::Down => {
              match app.tab_index {
                0 => {},
                1 => {},
                2 => {},
                3 => {app.topic_list_display_items.next()},
                _ => {},
              }
            },
            _ => {}
        }
    }
  }
  quit_application
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: VisualizatorApp, tick_rate: Duration) -> io::Result<()> {
  let mut last_tick = Instant::now();
  app.ros_participant.clear();
  loop {
      
      app.ros_participant.handle_node_read();
      app.set_topic_list_items();

      terminal.draw(|f| ui(f, &mut app))?;

      let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

      if handle_user_input(&mut app, &timeout)
      {
        return Ok(());
      }
        
      if last_tick.elapsed() >= tick_rate {
        last_tick = Instant::now();
      }
  }
}


fn ui<B: Backend>(f: &mut Frame<B>, app: &mut VisualizatorApp) {
  let size = f.size();
  let chunks = Layout::default()
      .direction(Direction::Vertical)
      .margin(5)
      .constraints([Constraint::Length(5),
                     Constraint::Length(3),
                     Constraint::Length(10)
                     ].as_ref())
      .split(size);

  let block = Block::default().style(Style::default().bg(Color::Black).fg(Color::White));
  f.render_widget(block, size);

  let help_strings = vec!("Change tab with arrow left and arrow right buttons.".to_string(),
    "Change selected item with arrow up and arrow down buttons".to_string(),
    "Quit application with 'q' button.".to_string(),);
  let user_help  = create_paragraph_from_string_list(help_strings," Usage Instructions".to_string());
  
  f.render_widget(user_help, chunks[0]);

  let titles = app
      .tab_titles
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
      .select(app.tab_index)
      .style(Style::default().fg(Color::Cyan))
      .highlight_style(
          Style::default()
              .add_modifier(Modifier::BOLD)
              .bg(Color::Black),
      );

  

  f.render_widget(tabs, chunks[1]);

  let first_row = create_layput_row(chunks[2]);

  let list_of_topics = List::new(app.topic_list_display_items.items.clone())
    .block(Block::default().title("Topics").borders(Borders::ALL))
    .style(Style::default().fg(Color::White))
    .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
    .highlight_symbol(">>");

  let list_of_local_nodes = List::new(app.local_nodes_display_items.items.clone())
  .block(Block::default().title("Local Nodes").borders(Borders::ALL))
  .style(Style::default().fg(Color::White))
  .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
  .highlight_symbol(">>");

  let list_of_external_nodes = List::new(app.external_nodes_display_items.items.clone())
  .block(Block::default().title("External Nodes").borders(Borders::ALL))
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

  let selected_topic_strings = match app.topic_list_display_items.state.selected(){
    Some (index) =>{
      get_topic_view_strings(&app.ros_participant, app.topic_list_items[index].topic_name())
    }
    None =>{vec!()}
  };

  let selected_topic_paragraph = create_paragraph_from_string_list(selected_topic_strings,"Topic information".to_string());
  

  let list_of_participants = List::new(participant_list_items)
  .block(Block::default().title("Participants").borders(Borders::ALL))
  .style(Style::default().fg(Color::White))
  .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
  .highlight_symbol(">>");

  match app.tab_index{
    0 => {f.render_widget(list_of_participants, first_row[0]);},
    1 => {
      f.render_stateful_widget(list_of_local_nodes, first_row[0], &mut app.local_nodes_display_items.state);
    }
    2 => {
      f.render_stateful_widget(list_of_external_nodes, first_row[0], &mut app.local_nodes_display_items.state);
    },
    3 => {
      f.render_stateful_widget(list_of_topics, first_row[0], &mut app.topic_list_display_items.state);
      f.render_widget(selected_topic_paragraph, first_row[1]);
    },
    4 => {},
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
    
  let ros_participant = RosParticipant::new().unwrap();
  let ros_node = ros_participant.new_ros_node("local_ node", "/ros2_demo", NodeOptions::new(true)).unwrap();
  
  let visualizor_app = VisualizatorApp::new(domain_participant,ros_participant,ros_node);

  let tick_rate = Duration::from_millis(250);
  let _res = run_app(&mut terminal, visualizor_app,tick_rate);

  disable_raw_mode()?;
  execute!(
      terminal.backend_mut(),
      LeaveAlternateScreen,
      DisableMouseCapture
  )?;
  terminal.show_cursor()?;  

  return Ok(());

}