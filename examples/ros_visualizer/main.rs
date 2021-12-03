use std::{
  io,
  error::Error,
  time::{Duration, Instant},
//  collections::{HashMap}
};

use log::{LevelFilter};
use log4rs::{
  append::console::ConsoleAppender,
  config::{Appender, Root},
  Config,
};
use structures::RosCommand;



mod stateful_list;
use crate::stateful_list::StatefulList;
mod display_string;
use crate::display_string::get_topics_list_view_strings;
use crate::display_string::get_topic_view_strings;
//use crate::display_string::get_external_node_info_strings;
use crate::display_string::get_node_list_strings;
use crate::display_string::get_participant_list_view_strings;
use crate::display_string::get_participant_view_strings;
use crate::display_string::get_node_view_strings;


mod visualization_helpers;
use crate::visualization_helpers::create_paragraph_from_string_list;
use crate::visualization_helpers::create_layput_row;

mod structures;
use crate::structures::DataUpdate;
//use crate::structures::NewROSParticipantFound;

use mio::{Events};
use mio::Token;
use mio::Ready;
use mio::PollOpt;
use mio::Poll;
use mio_extras::channel as mio_channel; 




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
//use rustdds::ros2::builtin_datatypes::Gid;
use rustdds::ros2::builtin_datatypes::ROSParticipantInfo;

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

const ROS2_COMMAND_TOKEN: Token = Token(1000);
const ROS2_NODE_RECEIVED_TOKEN: Token = Token(1001);


struct VisualizatorApp<'a> {
  receiver : mio_channel::Receiver<DataUpdate>,
  pub tab_titles: Vec<&'a str>,
  pub tab_index: usize,

  pub domain_participant : DomainParticipant,
  //pub ros_participant : RosParticipant,
  pub ros_node : RosNode,

  pub topic_list_items : Vec<DiscoveredTopicData>,
  pub external_nodes : Vec<NodeInfo>,
  pub local_nodes : Vec<NodeInfo>,
  pub ros_participants : Vec<ROSParticipantInfo>,
  //viewed datas:
  pub topic_list_display_items : StatefulList<ListItem<'a>>,
  pub external_nodes_display_items : StatefulList<ListItem<'a>>,
  pub local_nodes_display_items : StatefulList<ListItem<'a>>,
  pub ros_participants_display_items : StatefulList<ListItem<'a>>,
}

impl<'a> VisualizatorApp<'a> {
  fn new( domain_participant: DomainParticipant,  ros_node : RosNode, receiver : mio_channel::Receiver<DataUpdate>) -> VisualizatorApp<'a> {
    VisualizatorApp {

          receiver : receiver,
          tab_titles: vec!["Participants", "Local Nodes", "Exteral Nodes",  "Topics",],
          tab_index: 0,
          domain_participant,
          //ros_participant,
          ros_node,
          external_nodes : vec!(),
          topic_list_items : vec!(),
          local_nodes : vec!(),
          ros_participants : vec!(),
          topic_list_display_items : StatefulList::with_items(vec!()),
          external_nodes_display_items : StatefulList::with_items(vec!()),
          local_nodes_display_items : StatefulList::with_items(vec!()),
          ros_participants_display_items : StatefulList::with_items(vec!()),
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


  /*
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

  */
  /*
  pub fn set_node_list_items(&mut self) {
    let externals = self.ros_participant.get_all_discovered_external_ros_node_infos();
    let locals = self.ros_participant.get_all_discovered_local_ros_node_infos();

    self.local_nodes = locals.clone().into_iter().map(|(_string,nodes)|nodes).collect();
    
    for (_g,e_nodes) in externals {
      for node_i in e_nodes{
        self.external_nodes.push(node_i);
      }
    }

    let external_node_list_string = get_node_list_strings(&self.external_nodes);
    let local_node_list_string = get_node_list_strings(&self.local_nodes);
    let previous_state_external_nodes = self.external_nodes_display_items.state.clone();
    let previous_state_local_nodes = self.local_nodes_display_items.state.clone();
    
    self.external_nodes_display_items = StatefulList::with_items(vec!());
    for string in external_node_list_string {
      self.external_nodes_display_items.push(ListItem::new(string));
    }
    self.external_nodes_display_items.state = previous_state_external_nodes;

    self.local_nodes_display_items = StatefulList::with_items(vec!());
    for string in local_node_list_string {
      self.local_nodes_display_items.push(ListItem::new(string));
    }
    self.local_nodes_display_items.state = previous_state_local_nodes;

  }
   */

  pub fn set_discovered_topics(&mut self, topics : Vec<DiscoveredTopicData>){
    self.topic_list_items = topics;
    self.set_topics();
  }

  fn set_topics(&mut self){
    let topic_strings = get_topics_list_view_strings(&self.topic_list_items);
    let previous_state = self.topic_list_display_items.state.clone();
    self.topic_list_display_items = StatefulList::with_items(vec!());
    for string in topic_strings{
      self.topic_list_display_items.push(ListItem::new(string));
    }
    self.topic_list_display_items.state = previous_state;
  }

  pub fn add_new_ros_participant(&mut self, participant : ROSParticipantInfo){
    self.ros_participants.push(participant);
    self.set_ros_participants();
  }

  fn set_ros_participants(&mut self){
    let previous_state = self.ros_participants_display_items.state.clone();

    self.ros_participants_display_items = StatefulList::with_items(vec!());
    let display_strings = get_participant_list_view_strings(&self.ros_participants);
    for string in display_strings{
      self.ros_participants_display_items.push(ListItem::new(string))
    }
    self.ros_participants_display_items.state = previous_state;
  }

  pub fn set_node_infos(&mut self, nodes : Vec<NodeInfo>){
    self.local_nodes = nodes;
    self.set_nodes();
  }
   
  fn set_nodes(&mut self){

    let previous_state = self.local_nodes_display_items.state.clone();

    self.local_nodes_display_items = StatefulList::with_items(vec!());
    let display_strings = get_node_list_strings(&self.local_nodes);
    for string in display_strings{
      self.local_nodes_display_items.push(ListItem::new(string))
    }
    self.local_nodes_display_items.state = previous_state;

  }

  pub fn get_selected_participant_strings(&self) -> Vec<String>{
     match self.ros_participants_display_items.state.selected() {
      Some(index) => {
        match self.ros_participants.get(index){
          Some(item) =>{
            get_participant_view_strings(&item)
          }None =>{
            vec!()
          }
        }
      }
      None => {vec!()}
    }
  }

  
  pub fn get_selected_topic_strings(&self) -> Vec<String>{
    match self.topic_list_display_items.state.selected(){
      Some (index) =>{
        match self.topic_list_items.get(index){
          Some(item)=>{
            get_topic_view_strings(item)
          }None=>{vec!()}
        }
      }
      None =>{vec!()}
    }
  }
  

  pub fn get_selected_local_node_strings(&self) -> Vec<String>{
    match self.local_nodes_display_items.state.selected(){
      Some (index) =>{
        match self.local_nodes.get(index){
          Some(item)=>{
            get_node_view_strings(&item)
          }None=>{vec!()}
        }
      }
      None =>{vec!()}
    }
  }

  pub fn get_selected_external_node_strings(&self) -> Vec<String>{
    match self.external_nodes_display_items.state.selected(){
      Some (index) =>{
        match self.external_nodes.get(index){
          Some(item)=>{
            get_node_view_strings(&item)
          }None=>{vec!()}
        }
      }
      None =>{vec!()}
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
                0 => {app.ros_participants_display_items.previous()},
                1 => {app.local_nodes_display_items.previous() },
                2 => {app.external_nodes_display_items.previous()},
                3 => {app.topic_list_display_items.previous()},
                _ => {},
              }

            }, 
            KeyCode::Down => {
              match app.tab_index {
                0 => {app.ros_participants_display_items.next()},
                1 => {app.local_nodes_display_items.next() },
                2 => {app.external_nodes_display_items.next()},
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: VisualizatorApp, tick_rate: Duration, command_sender : mio_channel::SyncSender<RosCommand>) -> io::Result<()> {
  let mut last_tick = Instant::now();
  loop {

      //here check if ros thread has found new data and update it to VisualizatorApp
      loop{
        match &mut app.receiver.try_recv(){
          Ok(data) =>{
            match data {
              DataUpdate::NewROSParticipantFound { participant } => {&mut app.add_new_ros_participant(participant.clone())},
              DataUpdate::DiscoveredTopics {topics} => {& mut app.set_discovered_topics(topics.clone())} ,
              DataUpdate::DiscoveredNodes {nodes} => {& mut app.set_node_infos(nodes.clone())},
            };
          }
          //TODO HANDLE ERROR
          Err(_e) =>{
            break;
          }
        }
      }

      terminal.draw(|f| ui(f, &mut app))?;

      let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

      //If user presses quit button then send command to stop ros loop and exit from this loop also.
      if handle_user_input(&mut app, &timeout)
      {
        //TODO HANDLE RESULT
        let _send_command_result = command_sender.send(RosCommand::StopRosLoop);
        return Ok(());
      }
        
      if last_tick.elapsed() >= tick_rate {
        last_tick = Instant::now();
      }
  }
}

fn ros2_loop(sender : mio_channel::SyncSender<DataUpdate>, command_receiver : mio_channel::Receiver<RosCommand>){
  let mut ros_participant = RosParticipant::new().unwrap();
  //let mut ros_participant = &mut visualizator_app.ros_participant;
  let poll = Poll::new().unwrap();

  let mut update_timer = mio_extras::timer::Timer::default();
  update_timer.set_timeout(Duration::from_secs(1), ());

  poll
  .register(
    &ros_participant,
    ROS2_NODE_RECEIVED_TOKEN,
    Ready::readable(),
    PollOpt::edge(),
  )
  .unwrap();

  poll
      .register(
        &command_receiver,
        ROS2_COMMAND_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();


  loop{
    let mut events = Events::with_capacity(100);
    poll.poll(&mut events, None).unwrap();

    for event in events.iter() {
      if event.token() == ROS2_NODE_RECEIVED_TOKEN {
        let new_participants = ros_participant.handle_node_read();

        let node_infos = ros_participant.get_all_discovered_local_ros_node_infos().values().map(|x|x.clone()).collect();

        let _res_nodes_send = sender.send(
          DataUpdate::DiscoveredNodes{
              nodes : node_infos
            }
        );
      
        for participant in new_participants{
          let _res_participant_send = sender.send(
            DataUpdate::NewROSParticipantFound{
                participant : participant
              }
          );        

        }

        let new_topics = ros_participant.discovered_topics();
        let _res_topics_send = sender.send(
          DataUpdate::DiscoveredTopics{
            topics : new_topics
          }
        );
        
       
      }
      if event.token() == ROS2_COMMAND_TOKEN {
        match command_receiver.try_recv(){
          Ok(command) =>{
            match command {
                RosCommand::StopRosLoop => {return;},
            }
          //TODO HANDLE ERROR
          }Err(_e) =>{

          }
        }
      }
    }
  }
}


fn ui<B: Backend>(f: &mut Frame<B>, app: &mut VisualizatorApp) {
  let size = f.size();
  let chunks = Layout::default()
      .direction(Direction::Vertical)
      .margin(1)
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
 
  let list_of_participants = List::new(app.ros_participants_display_items.items.clone())
  .block(Block::default().title("Participants").borders(Borders::ALL))
  .style(Style::default().fg(Color::White))
  .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
  .highlight_symbol(">>");


  let selected_topic_paragraph = create_paragraph_from_string_list( app.get_selected_topic_strings(),"Topic information".to_string());
  let selected_participant_paragraph = create_paragraph_from_string_list(app.get_selected_participant_strings(),"Participant information".to_string());
  let selected_local_node_paragraph = create_paragraph_from_string_list(app.get_selected_local_node_strings(),"Node information".to_string());
  let selected_external_node_paragraph = create_paragraph_from_string_list(app.get_selected_external_node_strings(),"Node information".to_string());


  match app.tab_index{
    0 => {
      f.render_stateful_widget(list_of_participants, first_row[0], &mut app.ros_participants_display_items.state);
      f.render_widget(selected_participant_paragraph, first_row[1]);
    },
    1 => {
      f.render_stateful_widget(list_of_local_nodes, first_row[0], &mut app.local_nodes_display_items.state);
      f.render_widget(selected_local_node_paragraph, first_row[1]);
    }
    2 => {
      f.render_stateful_widget(list_of_external_nodes, first_row[0], &mut app.external_nodes_display_items.state);
      f.render_widget(selected_external_node_paragraph, first_row[1]);
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
  let mut ros_node = ros_participant.new_ros_node("local_node", "/ros2_demo", NodeOptions::new(true)).unwrap();
  ros_node.clear_node();

  //let (sender, receiver) = mio_channel::channel();
  let (sender, receiver) = mio_channel::sync_channel::<DataUpdate>(10);
  let (command_sender, command_receiver) = mio_channel::sync_channel::<RosCommand>(10);

  //ros2_loop(sender);
  let jhandle = std::thread::spawn(move || ros2_loop(sender,command_receiver));

  let visualizor_app = VisualizatorApp::new(domain_participant,ros_node,receiver);


 

  let tick_rate = Duration::from_millis(250);
  let _res = run_app(&mut terminal, visualizor_app,tick_rate, command_sender);

  jhandle.join().unwrap();

  disable_raw_mode()?;
  execute!(
      terminal.backend_mut(),
      LeaveAlternateScreen,
      DisableMouseCapture
  )?;
  terminal.show_cursor()?;  

  return Ok(());

}