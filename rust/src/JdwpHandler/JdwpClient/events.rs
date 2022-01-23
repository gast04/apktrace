#[path = "utils.rs"] mod utils;
#[path = "protocol_vars.rs"] mod pvars;

use crate::jdwp_handler::jdwp_client::connection::Connection as Conn;

pub struct Response {
  pub suspend_policy: u8,
  pub events: Vec<Event>,
}

impl Response {
  pub fn print(&self) {
    println!("Response:");
    println!("  Suspend Policy: {}", self.suspend_policy);
    for event in &self.events {
      event.print();
    }
  }
}

pub struct Event {
  pub kind: u8,
  pub type_tag: u8,
  pub request_id: u32,
  pub thread_id: u64,     // can be u32 or u64
  pub location: u64,
  pub class_id: u64,
  pub method_id: u64,
  pub retval: u64,
}

impl Event {
  pub fn print(&self) {
    println!("Event:");
    println!("  kind:       {}", self.kind);
    println!("  type tag:   {}", self.type_tag);
    println!("  request id: {}", self.request_id);
    println!("  thred id:   {}", self.thread_id);
    println!("  location:   {}", self.location);
    println!("  class id:   {}", self.class_id);
    println!("  method_id:  {}", self.method_id);
    println!("  retval:     {}", self.retval); // only print if WRV event
  }
}

pub fn parse_event_response(buffer: &[u8], obj_uid_size: u32) -> Response {

  let mut response = Response {
    suspend_policy: buffer[0],
    events: Vec::new(),
  };

  let event_cnt = utils::slice_to_u32(&buffer[1..5]);
  if event_cnt != 1 {
    println!("More than one event, not implemented!");
    return response;
  }

  /*let mut event = Event {
    event_kind: buffer[5],
    request_id: utils::slice_to_u32(&buffer[6..10]),
  }*/

  let event_kind = buffer[5];
  let request_id = utils::slice_to_u32(&buffer[6..10]);
  
  let mut thread_id: u64;
  if obj_uid_size == 4 {
    thread_id = utils::slice_to_u32(&buffer[10..14]) as u64;
  }
  else {
    thread_id = utils::slice_to_u64(&buffer[10..18]);
  }

  let it:usize = 10 + obj_uid_size as usize;
  
  /*
  let handle = match event_kind {
    pvars::EVENT_SINGLE_STEP => println!("event single step"),
    pvars::EVENT_BREAKPOINT  => println!("event breakpoint"),
    pvars::EVENT_METHOD_ENTRY => println!("event method entry"),
    pvars::EVENT_METHOD_EXIT => println!("event method exit"),
    pvars::EVENT_METHOD_EXIT_WRV => println!("event method exit WRV"),
    _ => println!("Unknown Event kind: {}", event_kind),
  }*/

  if event_kind == pvars::EVENT_METHOD_ENTRY ||
     event_kind == pvars::EVENT_METHOD_EXIT_WRV {

    let type_tag = buffer[it];

    // important for breakpoint and method name parsing
    let class_id = utils::slice_to_u64(&buffer[it+1..it+9]);
    let method_id = utils::slice_to_u64(&buffer[it+9..it+17]);
    let loc_index = utils::slice_to_u64(&buffer[it+17..it+25]);

    let it = it+25;
    
    let mut retval: u64 = 0;
    if event_kind == pvars::EVENT_METHOD_EXIT_WRV {
      
      retval = match buffer[it] {
        86 => 0,                                                  /*V void */
        90 => buffer[it+1] as u64,                                /*Z bool */
        73 => utils::slice_to_u32(&buffer[it+1..it+5]) as u64,    /*I Integer */
        74 => utils::slice_to_u64(&buffer[it+1..it+9]),           /*J Long */
        76 => 0,                                                  /*L Object */
        _ =>  0,
      };
    }

    let event = Event {
      kind: event_kind,
      request_id: request_id,
      thread_id: thread_id,
      type_tag: type_tag,
      class_id: class_id,
      method_id: method_id,
      location: loc_index,
      retval: retval,
    };
    response.events.push(event);
  }

  return response;
}

pub fn class_match_event(con: &mut Conn,
    class_pattern: &str, event_kind: u8) -> u32 {
  /*
    Packet data Format:
      B Event Kind
      B Suspend policy
      I modifiers
      [
        -> modifer depends on type

        B modifier kind (5) ClassMatch
        S classPattern
      ]
    all modifiers have to match, so classmatch allows
    only one
  */

  // create packet data
  let mut packet_data:Vec<u8> = Vec::new();
  packet_data.push(event_kind);
  packet_data.push(pvars::SUSPEND_ALL);
  utils::append_u32(&mut packet_data, 1); // add modifier
  packet_data.push(pvars::MODKIND_CLASS_MATCH);
  utils::append_string(&mut packet_data, class_pattern);

  con.send_packet(pvars::EVENTSET_SIG, &packet_data);
  return con.read_reqid();
}

pub fn class_exclude_event(con: &mut Conn, event_kind: u8) -> u32 {
  /*
    Packet data Format:
      B Event Kind
      B Suspend policy
      I modifiers
      [
        -> modifer depends on type

        B modifier kind (6) ClassExclude
        S classPattern
      ]
  */

  // create packet data
  let mut packet_data:Vec<u8> = Vec::new();
  packet_data.push(event_kind);
  packet_data.push(pvars::SUSPEND_ALL);
  utils::append_u32(&mut packet_data, 9); // add modifier

  // unwanted java classes
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "java.*");
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "javax.*");
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "sun.*");
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "com.sun.*");

  // android specific classes
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "dalvik.system.*");
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "libcore.*");
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "android.*");
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "com.android.*");
  packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
  utils::append_string(&mut packet_data, "androidx.*");

  con.send_packet(pvars::EVENTSET_SIG, &packet_data);
  return con.read_reqid();
}
