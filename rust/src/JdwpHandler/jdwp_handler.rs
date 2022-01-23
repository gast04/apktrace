#[path = "JdwpClient/jdwp_client.rs"] mod jdwp_client;


// protvars can be consts, thats fine
//const THING: u32 = 0xABAD1DEA;

pub struct JdwpHandler {
  client: jdwp_client::JdwpClient,
  verbose: bool,
}

// add functions to the struct
impl JdwpHandler {
  pub fn new(host: &str, port: u16, verbose: bool) -> Result<Self, i8> {

    let res = jdwp_client::JdwpClient::new(host, port, verbose);
    if !res.is_ok() {
      println!("[JdwpHandler] Could not create Client!");
      return Err(-1);
    }
    let client = res.unwrap();
    return Ok(JdwpHandler { client: client, verbose: verbose });
  }

}

pub fn init_connection(host: &str, port: u16, verbose: bool)
  -> Result<JdwpHandler, i8> {

  // create initial tcp connection
  let res = JdwpHandler::new(host, port, verbose);
  if !res.is_ok() {
    return Err(-1);
  }
  let mut _handler = res.unwrap();

  // perform basic startup commands
  _handler.client.handshake();
  _handler.client.suspend_vm();
  _handler.client.get_idsizes();
  _handler.client.get_version();
  _handler.client.fetch_classes();

  _handler.client.print_version();

  return Ok(_handler);
}


pub fn break_on_method_entry(handler: &mut JdwpHandler, class_pattern: &str) {
  //handler.client.evt_entry_class_match(class_pattern);
  handler.client.evt_entry_class_exclude();
}

pub fn break_on_method_exit_wrv(handler: &mut JdwpHandler, class_pattern: &str) {
  //handler.client.evt_exit_wrv_class_match(class_pattern);
  handler.client.evt_exit_wrv_class_exclude();
}

pub fn resume_vm(handler: &mut JdwpHandler) {
  handler.client.resume_vm();
}

pub fn wait_for_event(handler: &mut JdwpHandler) {
  handler.client.wait_for_event();
}
