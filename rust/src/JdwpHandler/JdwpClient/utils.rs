#[path = "protocol_vars.rs"] pub mod pvars;

use std::str;
use std::convert::TryInto;
use chrono::{Timelike, DateTime, Utc};

use colored::*;
use crate::jdwp_handler::jdwp_client::connection::Connection as Conn;

pub struct IdSizes {
  pub field_id_size: u32,
  pub method_id_size: u32,
  pub object_id_size: u32,
  pub reference_type_id_size: u32,
  pub frame_id_size: u32,
}
impl IdSizes {

  pub fn new() -> Self {
    return IdSizes {
      field_id_size:0,
      method_id_size:0,
      object_id_size:0,
      reference_type_id_size:0,
      frame_id_size:0
    };
  }

  pub fn print(&self) {
    println!("IdSizes:");
    println!("  Field:    {}", self.field_id_size);
    println!("  Method:   {}", self.method_id_size);
    println!("  Object:   {}", self.object_id_size);
    println!("  Ref Type: {}", self.reference_type_id_size);
    println!("  Frame:    {}", self.frame_id_size);
  }
}

pub struct Version {
  pub description: String,
  pub major: u32,
  pub minor: u32,
  pub vm_version: String,
  pub vm_name: String,
}
impl Version {
  pub fn new() -> Self {
    return Version {
      description: "".to_string(),
      major: 0,
      minor: 0,
      vm_version: "".to_string(),
      vm_name: "".to_string(),
    }
  }
  pub fn print(&self) {
    println!("Version:");
    println!("  Description: {}", self.description);
    println!("  Major/Minor: {}/{}", self.major, self.minor);
    println!("  Vm Version: {}", self.vm_version);
    println!("  Vm Name: {}", self.vm_name);
  }
}

pub struct Threads {
  pub ids: Vec<u64>,
  pub names: Vec<String>,
  pub colors: Vec<u8>,
  pub current_color: u8,
}
impl Threads {
  pub fn new() -> Self {
    return Threads {
      ids: Vec::new(),
      names: Vec::new(),
      colors: Vec::new(),
      current_color: 0,
    }
  }
  pub fn next_color(&mut self) -> u8 {
    let ncolor = self.current_color;

    self.current_color += 1;
    if self.current_color > 4 {
      self.current_color = 0; // loop if there are too many threads
    }

    return ncolor;
  }
}

pub fn thread_str(thread_id: u64, thread_name: String, color: u8) -> String {

  let mut tstr = format!("[{}: {}]", thread_name, thread_id);
  
  tstr = match color {
    0 => tstr.cyan().to_string(),
    1 => tstr.blue().to_string(),
    2 => tstr.magenta().to_string(),
    3 => tstr.yellow().to_string(),
    4 => tstr.white().to_string(),
    _ => tstr.truecolor(169, 30, 204).to_string(),
  };

  return tstr;
}

pub fn slice_to_u32(slice: &[u8]) -> u32 {
  return u32::from_be_bytes(slice.try_into().unwrap());
}

pub fn slice_to_u64(slice: &[u8]) -> u64 {
  return u64::from_be_bytes(slice.try_into().unwrap());
}

pub fn parse_string(buffer:  &[u8]) -> String {
  let str_length: usize = slice_to_u32(&buffer[0..4]) as usize;
  let str_raw = &buffer[4..(str_length+4)];
  let res = str::from_utf8(str_raw);
  if !res.is_ok() {
    return "".to_string();
  }

  return res.unwrap().to_string();
}

pub fn append_u32(buffer: &mut Vec<u8>, value: u32) {
  let mods_vec = value.to_be_bytes();
  for i in 0..4 { buffer.push(mods_vec[i]); }
}

pub fn append_u64(buffer: &mut Vec<u8>, value: u64) {
  let mods_vec = value.to_be_bytes();
  for i in 0..8 { buffer.push(mods_vec[i]); }
}

pub fn append_string(buffer: &mut Vec<u8>, class_pattern: &str) {

  append_u32(buffer, class_pattern.len() as u32);

  for b in class_pattern.as_bytes() {
    buffer.push(*b);
  }
}

pub fn parse_idsizes(buffer: &Vec<u8>) -> IdSizes {
  /*
    Packet reply format:
    ("I", "fieldIDSize")
    ("I", "methodIDSize")
    ("I", "objectIDSize")
    ("I", "referenceTypeIDSize")
    ("I", "frameIDSize")
  */

  let mut idsizes = IdSizes::new();
  idsizes.field_id_size = slice_to_u32(&buffer[0..4]);
  idsizes.method_id_size = slice_to_u32(&buffer[4..8]);
  idsizes.object_id_size = slice_to_u32(&buffer[8..12]);
  idsizes.reference_type_id_size = slice_to_u32(&buffer[12..16]);
  idsizes.frame_id_size = slice_to_u32(&buffer[16..20]);
  return idsizes;
}

pub fn parse_version(buffer: &Vec<u8>) -> Version {
  /*
    Packet reply format:
    ('S', "description")
    ('I', "jdwpMajor")
    ('I', "jdwpMinor")
    ('S', "vmVersion")
    ('S', "vmName")
  */

  let mut version = Version::new();
  let desc = parse_string(buffer);
  version.description = desc.clone();

  let it = 4+desc.len();
  version.major = slice_to_u32(&buffer[(it)..(it+4)]);
  version.minor = slice_to_u32(&buffer[(it+4)..(it+8)]);

  let vm_version = parse_string(&buffer[(it+8)..]);
  version.vm_version = vm_version.clone();

  let it = it + 8 + 4 + vm_version.len();
  let vm_name = parse_string(&buffer[it..]);
  version.vm_name = vm_name.clone();

  return version;
}

pub fn get_current_time() -> String {
  let now: DateTime<Utc> = Utc::now();
  let time = format!("{:02}:{:02}:{:02}-{:03}", (now.hour()+1)%24,
      now.minute(), now.second(), now.timestamp_millis() % 1000);
  return time;
}

pub fn get_thread_by_id(con: &mut Conn,
    obj_id_size: u32, thread_id: u64) -> String {

  let mut data: Vec<u8> = Vec::new();
  if obj_id_size == 4 {
    append_u32(&mut data, thread_id as u32);
  } else {
    append_u64(&mut data, thread_id);
  }

  con.send_packet(pvars::THREADNAME_SIG, &data);
  return con.read_string();
}
