use rustdds::ros2::RosParticipant;
use rustdds::dds::data_types::DiscoveredTopicData;
use rustdds::ros2::builtin_datatypes::NodeInfo;



/*
pub fn clear_screen(){
  print!("{}[2J", 27 as char);
  print!("\x1B[2J\x1B[1;1H");
}

pub fn print_datas(participant : &RosParticipant){
  clear_screen();

  println!("This application participant: ");
  print_local_participant_info(participant);
  println!("");
  println!("All topics:");
  print_topics(participant);

  println!("");
  println!("External nodes:");
  print_external_node_infos(participant);
}


pub fn print_topics(participant : &RosParticipant){
  
  let strings = get_topics_list_view_strings(&participant.discovered_topics());
  for s in strings{
    println!("  {:?}",s);
  }
}


pub fn print_local_participant_info(participant : &RosParticipant){
  let strings = get_local_ros_participant_info_strings(participant);
  for s in strings{
    println!("  {:?}",s);
  }
}

pub fn print_external_node_infos(participant : &RosParticipant){
  let strings = get_external_node_info_strings(participant);
  for s in strings{
    println!("  {:?}",s);
  }
}
*/

pub fn get_local_ros_participant_info_strings(participant : &RosParticipant) -> Vec<String>{
  let info = participant.get_ros_participant_info();
  let mut strings = vec!();
  strings.push(format!("guid: {:?}, nodes: {:?}", info.guid(), info.nodes() ));
  strings
}



pub fn get_topics_list_view_strings(discovered_topic_datas : &Vec<DiscoveredTopicData>) -> Vec<String>{
  let mut strings = vec!();
  for topic in discovered_topic_datas{
    strings.push(format!("{:?}",topic.topic_name()));
  }
  strings
}

pub fn get_topic_view_strings(participant : &RosParticipant, topic_name : &String) -> Vec<String>{
  let topics = participant.discovered_topics();
  let mut strings = vec!();
  match topics.into_iter().find(|x| x.topic_name() == topic_name){
      //match ok{
        Some(topic) =>{
          strings.push(format!("name: {:?}", topic.topic_name()));
          strings.push(format!("type_name: {:?}", topic.type_name()));
          strings.push(format!("durability: {:?}", topic.topic_data.durability));
          strings.push(format!("deadline: {:?}", topic.topic_data.deadline));
          strings.push(format!("latency_budget: {:?}", topic.topic_data.latency_budget));
          strings.push(format!("liveliness: {:?}", topic.topic_data.liveliness));
          strings.push(format!("reliability: {:?}", topic.topic_data.reliability));
          strings.push(format!("lifespan: {:?}", topic.topic_data.lifespan));
          strings.push(format!("destination_order: {:?}", topic.topic_data.destination_order));
          strings.push(format!("presentation: {:?}", topic.topic_data.presentation));
          strings.push(format!("history: {:?}", topic.topic_data.history));
          strings.push(format!("resource_limits: {:?}", topic.topic_data.resource_limits));
          strings.push(format!("ownership: {:?}", topic.topic_data.ownership));  
        }
        None => {}
      }
  strings
}

pub fn get_node_list_strings(nodes : &Vec<NodeInfo>) -> Vec<String>{
  let mut strings = vec!();
  for node in nodes{
    strings.push(format!("{:?}{:?}", node.namespace(), node.name()));
  }
  strings
}

pub fn get_external_node_info_strings(participant : &RosParticipant) -> Vec<String> {
  let node_infos = participant.get_all_discovered_external_ros_node_infos();
  let mut strings = vec!();
  for (_gid, info_vec) in node_infos {
    for node_info in info_vec{
      strings.push(format!("name: {:?} namespace {:?} ",node_info.name(),node_info.namespace()   ));
    }
  }
  strings
}


