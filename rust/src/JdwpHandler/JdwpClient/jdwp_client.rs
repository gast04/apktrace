#[path = "Connection/connection.rs"] mod connection;
#[path = "utils.rs"] mod utils;
#[path = "protocol_vars.rs"] mod pvars;
#[path = "classes.rs"] mod class;
#[path = "methods.rs"] mod method;
#[path = "events.rs"] mod event;

// simplify namespace of Connection module
use connection::Connection as Conn;

use colored::*;

pub struct JdwpClient {
  pub con: Conn,
  pub idsizes: utils::IdSizes,
  pub classes: class::Classes,
  pub methods: method::Methods,
  pub threads: utils::Threads,
  pub created_events: Vec<(u8, u32)>, // (Event kind, request id)
  pub version: utils::Version,
  verbose: bool,
}

// add functions to the struct
impl JdwpClient {
  pub fn new(host: &str, port: u16, verbose: bool) -> Result<Self, i8> {

    let res = Conn::new(host, port, verbose);
    if !res.is_ok() {
      println!("[JdpwClient] Could not connect to server!");
      return Err(-1);
    }
    let con = res.unwrap();
    return Ok(JdwpClient { con: con,
        idsizes: utils::IdSizes::new(),
        classes: class::Classes::new(),
        methods: method::Methods::new(),
        threads: utils::Threads::new(),
        created_events: Vec::new(),
        version: utils::Version::new(),
        verbose: verbose 
      });
  }

  fn dbg_print(&self, msg: &str) {
    if self.verbose {
      println!("[JdwpClient] {}",msg);
    }
  }

  pub fn get_version(&mut self) {
    self.dbg_print("get_version");
    self.con.send_packet(pvars::VERSION_SIG, b"");
    let res = self.con.read_buffer();
    if !res.is_ok() {
      self.dbg_print("could not fetch version");
      return;
    }

    let data_buffer = res.unwrap();
    self.version = utils::parse_version(&data_buffer);
    if self.verbose {
      self.version.print();
    }
  }

  pub fn print_version(&mut self) {
    let parts = self.version.description.split("\n");
    for s in parts {
      println!("{} {}", "[JdwpClient]".yellow(), s);
    }
  }

  pub fn handshake(&mut self) -> bool {
    self.dbg_print("handshake");

    let handshake_str = b"JDWP-Handshake";
    if !self.con.send_raw(handshake_str) {
      return false;
    }

    let mut buffer: [u8; 14] = [0; 14];
    let recv = self.con.read_raw(&mut buffer);

    if !buffer.eq(handshake_str) {
      self.dbg_print("Handshake failed");
      return false;
    }

    return true;
  }

  pub fn suspend_vm(&mut self) {
    self.dbg_print("suspend_vm");
    self.con.send_packet(pvars::SUSPENDVM_SIG, b"");
    self.con.wait_reply();
  }

  pub fn resume_vm(&mut self) {
    self.dbg_print("resume_vm");
    self.con.send_packet(pvars::RESUMEVM_SIG, b"");
    self.con.wait_reply();
  }

  pub fn get_idsizes(&mut self) {
    self.dbg_print("get_idsizes");
    self.con.send_packet(pvars::IDSIZES_SIG, b"");

    let res = self.con.read_buffer();
    if !res.is_ok() {
      self.dbg_print("could not fetch ID-sizes");
      return;
    }

    let data_buffer = res.unwrap();
    self.idsizes = utils::parse_idsizes(&data_buffer);
    if self.verbose {
      self.idsizes.print();
    }
  }

  pub fn fetch_classes(&mut self) {
    self.dbg_print("fetch_classes");

    class::fetch_all_classes(&mut self.con,
      &mut self.classes, self.idsizes.reference_type_id_size);

    self.dbg_print(&format!("Fetched classes: {}", self.classes.vec.len()));
  }

  pub fn evt_entry_class_match(&mut self, class_pattern: &str) {
    self.dbg_print("evt_entry_class_match");

    let e_kind = pvars::EVENT_METHOD_ENTRY;
    let r_id = event::class_match_event(&mut self.con, &class_pattern, e_kind);
    self.created_events.push((e_kind, r_id));
  }

  pub fn evt_entry_class_exclude(&mut self) {
    self.dbg_print("evt_entry_class_exclude");

    let e_kind = pvars::EVENT_METHOD_ENTRY;
    let r_id = event::class_exclude_event(&mut self.con, e_kind);
    self.created_events.push((e_kind, r_id));
  }

  pub fn evt_exit_wrv_class_match(&mut self, class_pattern: &str){
    self.dbg_print("evt_exit_wrv_class_match");

    let e_kind = pvars::EVENT_METHOD_EXIT_WRV;
    let r_id = event::class_match_event(&mut self.con, &class_pattern, e_kind);
    self.created_events.push((e_kind, r_id));
  }

  pub fn evt_exit_wrv_class_exclude(&mut self){
    self.dbg_print("evt_exit_wrv_class_exclude");

    let e_kind = pvars::EVENT_METHOD_EXIT_WRV;
    let r_id = event::class_exclude_event(&mut self.con, e_kind);
    self.created_events.push((e_kind, r_id));
  }

  pub fn wait_for_event(&mut self) {

    let res = self.con.read_buffer();
    if !res.is_ok() {
      return;
    }

    // parse response event buffer
    let buffer = res.unwrap();
    let response = event::parse_event_response(&buffer, self.idsizes.object_id_size);
    if self.verbose {
      response.print();
    }

    let event = &response.events[0];

    // fetch class_name and method name
    // TODO: how to remove these many params?
    let class_name = class::get_name_by_id(&mut self.con, &mut self.classes,
        self.idsizes.reference_type_id_size, event.class_id);

    // change to get method by id, as we can read everything from the method struct
    // add "has_retval" to method
    let method = method::get_method_by_id(&mut self.con,
        &mut self.methods,
        self.idsizes.method_id_size, self.idsizes.object_id_size,
        event.class_id, event.method_id);

    let mut method_name = method.name.clone() + &method.signature;
    if method.native {
      method_name = method_name.yellow().to_string();
    }

    let mut thread_name:String = "".to_string();
    let mut thread_color = 0;
    for i in 0..self.threads.ids.len() {
      if self.threads.ids[i] == event.thread_id {
        thread_name = self.threads.names[i].clone();
        thread_color = self.threads.colors[i];
      }
    }

    if thread_name.len() == 0 {
      thread_name = utils::get_thread_by_id(&mut self.con,
          self.idsizes.object_id_size, event.thread_id);
      thread_color = self.threads.next_color();

      // cache created values
      self.threads.ids.push(event.thread_id);
      self.threads.names.push(thread_name.clone());
      self.threads.colors.push(thread_color);
    }

    // pretty print, move to jdwp handler
    for (kind, r_id) in &self.created_events {

      if event.request_id != *r_id {
        continue;
      }

      let thread_str = utils::thread_str(event.thread_id,
          thread_name.clone(), thread_color);

      let time = utils::get_current_time();
      print!("[{}] ", time);
        
      if *kind == pvars::EVENT_METHOD_ENTRY {
        print!("{} ", "Entry".green());
      }
      else if *kind == pvars::EVENT_METHOD_EXIT_WRV{
        print!("{}  ", "Exit".red());
      }

      print!("{} ", thread_str);
      if method.native {
        print!("N ");
      }
      else {
        print!("  ");
      }
      print!("{} -> {}", class_name, method_name);
      
      /*
      // TODO: fix this implementation
      if *kind == pvars::EVENT_METHOD_EXIT_WRV && !method.ret_void {
        // non void, print return value
        print!(" {}", event.retval.to_string().magenta());
      }
      */
      println!("");
    }
  }
}
