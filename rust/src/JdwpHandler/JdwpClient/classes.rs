#[path = "utils.rs"] mod utils;
#[path = "protocol_vars.rs"] mod pvars;

use crate::jdwp_handler::jdwp_client::connection::Connection as Conn;

pub struct Class {
  pub ref_type_tag: u8,
  pub ref_type_id: u64, // either u32 or u64
  pub signature: String,
  pub status: u32,
}
impl Class {
  pub fn new() -> Self {
    return Class {
      ref_type_tag:0,
      ref_type_id:0,
      signature:"".to_string(),
      status:0
    };
  }

  pub fn print(&self) {
    println!("Class:");
    println!("  Ref Type Tag: {}", self.ref_type_tag);
    println!("  Ref Typ Id:   {}", self.ref_type_id);
    println!("  Signature:    {}", self.signature);
    println!("  Status:       {}", self.status)
  }
}

pub struct Classes {
  pub vec: Vec<Class>
}
impl Classes {
  pub fn new() -> Self {
    return Classes {
      vec: Vec::new(),
    }
  }
}

fn parse_class(slice: &[u8], tis: u32) -> (Class, u32) {
  /*
    Packet reply format:
    ('C', "refTypeTag")
    (reference_type_id_size, "refTypeId")
    ('S', "signature")
    ('I', "status")
  */

  let mut class = Class::new();
  class.ref_type_tag = slice[0];
  if tis == 4 {
    class.ref_type_id = (utils::slice_to_u32(&slice[1..5]) as u32).into();
  }
  else {
    class.ref_type_id = utils::slice_to_u64(&slice[1..9]);
  }

  let str_start = 1 + tis as usize;
  let signature = utils::parse_string(&slice[str_start..]);

  let it = str_start + 4 + signature.len();
  class.signature = signature.clone();
  class.status = utils::slice_to_u32(&slice[it..it+4]);

  return (class, (it+4) as u32);
}

pub fn fetch_all_classes(con: &mut Conn,
    classes: &mut Classes, type_id_size: u32) {

  con.send_packet(pvars::ALLCLASSES_SIG, b"");
  let res = con.read_buffer();
  if !res.is_ok() {
    // could not read buffer
    return;
  }
  let buffer = res.unwrap();

  // number of entries as I, and after list of classes
  let cnt = utils::slice_to_u32(&buffer[0..4]);

  let mut next_entry: u64 = 0;
  for _ in 0..cnt {
    let class = parse_class(&buffer[(4+next_entry as usize)..], type_id_size);
    // append class to classes list, only if signature
    if class.0.signature.len() != 0 {
      classes.vec.push(class.0);
    }
    next_entry += class.1 as u64;
  }
}

pub fn get_name_by_id(con: &mut Conn, classes: &mut Classes,
    ref_tis: u32 ,ref_type_id: u64) -> String {

  for class in &classes.vec {
    if class.ref_type_id == ref_type_id {
      return class.signature.clone();
    }
  }

  // clear all classes and reload again
  classes.vec.clear();
  fetch_all_classes(con, classes, ref_tis);
  for class in &classes.vec {
    if class.ref_type_id == ref_type_id {
      return class.signature.clone();
    }
  }

  return "unknownClass;".to_string();
}