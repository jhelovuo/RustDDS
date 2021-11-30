
//mod display_string{
use rustdds::ros2::RosParticipant;



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
  let strings = get_topics_view_strings(participant);
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

pub fn get_local_ros_participant_info_strings(participant : &RosParticipant) -> Vec<String>{
  let info = participant.get_ros_participant_info();
  let mut strings = vec!();
  strings.push(format!("guid: {:?}, nodes: {:?}", info.guid(), info.nodes() ));
  strings
}

pub fn get_topics_view_strings(participant : &RosParticipant) -> Vec<String>{
  let topics = participant.discovered_topics();
  let mut strings = vec!();
  for topic in topics{
    strings.push(format!("name: {:?} type: {:?} ",topic.topic_name(), topic.type_name()));
  }
  strings
}

pub fn get_external_node_info_strings(participant : &RosParticipant) -> Vec<String> {
  let node_infos = participant.get_all_discovered_external_ros_node_infos();
  let mut strings = vec!();
  for (gid, info_vec) in node_infos {
    for node_info in info_vec{
      strings.push(format!("name: {:?} namespace {:?} ",node_info.name(),node_info.namespace()   ));
    }
  }
  strings
}


//}