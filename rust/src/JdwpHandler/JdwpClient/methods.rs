#[path = "protocol_vars.rs"] pub mod pvars;
#[path = "utils.rs"] mod utils;
//#[path = "Connection/connection.rs"] mod connection;

// chain of modules, starting from root directory
use crate::jdwp_handler::jdwp_client::connection::Connection as Conn;
// a module should be only defined once, otherwise casts between
// the two different modules are not allowed
// which makes sense as it can be different modules just with the
// same name

//use crate::jdwp_handler::jdwp_client::JdwpClient as Jclient;


pub struct Method {
  pub ref_type_id: u64, // either u32 or u64
  pub method_id: u64,
  pub name: String,
  pub signature: String,
  pub modbits: u32,
  pub ret_void: bool,     // return nothing
  pub native: bool,
}
impl Method {
  pub fn new() -> Self {
    return Method {
      ref_type_id: 0,    // class_id
      method_id: 0,
      name: "unknown".to_string(),
      signature: "()V".to_string(),
      modbits: 0,
      ret_void: true,
      native: false,
    };
  }

  pub fn print(&self) {
    println!("Method:");
    println!("  Ref Type Id: {}", self.ref_type_id);
    println!("  Method Id:   {}", self.method_id);
    println!("  Name:        {}", self.name);
    println!("  Signature:   {}", self.signature);
    println!("  Modbits:     {}", self.modbits);
    println!("  Native:      {}", self.native);
  }
  pub fn copy(&self) -> Method {
    return Method{
      ref_type_id: self.ref_type_id,
      method_id: self.method_id,
      name: self.name.clone(),
      signature: self.signature.clone(),
      modbits: self.modbits,
      ret_void: self.ret_void,
      native: self.native,
    }
  }
}

pub struct Methods {
  pub vec: Vec<Method>
}
impl Methods {
  pub fn new() -> Self {
    return Methods {
      vec: Vec::new(),
    }
  }
  pub fn print(&self) {
    println!("Methods:");
    for m in &self.vec {
      m.print();
    }
  }
}

/*
// class_id, method_id
pub fn fetch_methods_packet(obj_id_size: u32,
        meth_id_size: u32,
        ref_type_id: u64) -> Vec<u8> {

  // fill data with class_id from which we wanna fetch all methods
  let mut data: Vec<u8> = Vec::new();
  let rti_vec = ref_type_id.to_be_bytes();
  if obj_id_size == 4 {
    for i in 0..4 { data.push(rti_vec[i]); }
  }
  else {
    for i in 0..8 { data.push(rti_vec[i]); }
  }

  return data;
}
*/

fn parse_method(buffer: &[u8], rti: u64, m_id_size: u32) -> (Method, usize) {

  /*
    Method Struct:
      m_id_size: method_id
      S: name
      S: signature
      I: modBits
  */

  let mut it  = 0;
  let mut method_id: u64 = 0;
  if m_id_size == 4 {
    method_id = utils::slice_to_u32(&buffer[it..it+4]) as u64;
  }
  else {
    method_id = utils::slice_to_u64(&buffer[it..it+8]);
  }
  it += m_id_size as usize;

  let name = utils::parse_string(&buffer[it..]);
  it += name.len() + 4;

  let signature = utils::parse_string(&buffer[it..]);
  it += signature.len() + 4;

  let modbits = utils::slice_to_u32(&buffer[it..it+4]);
  it += 4;

  // create method struct and return
  let method = Method {
    ref_type_id: rti,
    method_id: method_id,
    name: name.clone(),
    signature: signature.clone(),
    modbits: modbits,
    ret_void: *signature.as_bytes().last().unwrap() == 86, // "V"
    native: modbits & 0x0100 > 0,
  };

  return (method, it);
}


pub fn get_method_by_id(con: &mut Conn, methods: &mut Methods,
    m_id_size: u32, o_id_size: u32,
    ref_type_id: u64, method_id: u64) -> Method {

  // check if method name is already in vector
  for method in &methods.vec {
    if method.ref_type_id == ref_type_id &&
        method.method_id == method_id as u64 {
          return method.copy();
    }
  }

  // method not found fetch all methods from this class

  // fill data with class_id from which we wanna fetch all methods
  let mut data: Vec<u8> = Vec::new();
  if o_id_size == 4 {
    utils::append_u32(&mut data, ref_type_id as u32);
  } else {
    utils::append_u64(&mut data, ref_type_id);
  }
  con.send_packet(pvars::METHODS_SIG, &data);

  // read and parse result
  let res = con.read_buffer();
  if !res.is_ok() {
    println!("Could not read method buffer");
    return Method::new();
  }
  let buffer = res.unwrap();

  let cnt = utils::slice_to_u32(&buffer[0..4]);

  let mut it: usize = 4;
  for _ in 0..cnt {
    let (method, nit) = parse_method(&buffer[it..], ref_type_id, m_id_size);
    it += nit;
    methods.vec.push(method);
  }

  for method in &methods.vec {
    if method.ref_type_id == ref_type_id &&
        method.method_id == method_id as u64 {
          return method.copy();
    }
  }

  return Method::new();
}
