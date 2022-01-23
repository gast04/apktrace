use std::process;

// https://github.com/tailhook/rust-argparse
use argparse::{ArgumentParser, Store, Print};

#[path = "Utils/apktools.rs"] mod apktools;
#[path = "JdwpHandler/jdwp_handler.rs"] mod jdwp_handler;

// this key is different for each customer, if a version leaks
// we can trace it back
pub static DRM_KEY: u64 = 0x9a3298afb5ac71c7;

fn main() {
  println!("Thanks for using apktrace!");

  let mut target_pid:u64 = DRM_KEY;
  {  // this block limits scope of borrows by ap.refer() method
    let mut ap = ArgumentParser::new();
    ap.set_description("apktrace by niku.systems");
    ap.refer(&mut target_pid)
      .add_argument("pid", Store,
      "Process pid (on device)");
    ap.add_option(&["-v", "--version"],
      Print("Version: 3.0.0".to_string()), "Show version");
    ap.parse_args_or_exit();
  }
  if target_pid == DRM_KEY {
    println!("No pid given!");
    std::process::exit(-1);
  }

  let tcp_port = 33333;
  apktools::forward_jdwp(tcp_port, target_pid);

  // TODO: adb shell ps is not the same everywhere so skip in first version
  //let _package_id:String = "com.sample.demo_keystore2".to_string();
  //let _activity:String = "com.sample.demo_keystore.MainActivity".to_string();
  //apktools::start_application(&package_id, &activity);

  let res = jdwp_handler::init_connection("127.0.0.1", tcp_port as u16, false);
  if !res.is_ok() {
    process::exit(-1);
  }
  let mut _handler = res.unwrap();


  //println!("Set Method Entry break!");
  // this is not used yet, as the exclude class match condition is used
  jdwp_handler::break_on_method_entry(&mut _handler, "com.sam*");
  jdwp_handler::break_on_method_exit_wrv(&mut _handler, "com.sam*");
  
  jdwp_handler::resume_vm(&mut _handler);

  //println!("start waiting for events!");
  loop {
    jdwp_handler::wait_for_event(&mut _handler);
    jdwp_handler::resume_vm(&mut _handler);
  }
}
