use std::process::Command;

pub fn start_application(package_id: &String, activity: &String) -> Result<bool, String> {
  println!("starting application");
  // adb shell am start -D -n com.sample.demo_keystore2/com.sample.demo_keystore.MainActivity

  let mut start_str: String = String::from(package_id);
  start_str.push('/');
  start_str.push_str(activity);

  //println!("Starting: {:?}",start_str);

  Command::new("adb")
    .arg("shell")
    .arg("am start -D -n")
    .arg(start_str)
    .output()
    .expect("failed to execute process");

  return Err("Hell no".to_string());
}

pub fn forward_jdwp(tcp_port: u64, pid: u64) {
  // adb forward tcp:33333 jdwp:11220

  println!("[apktrace] adb forward tcp:{} jdwp:{}", tcp_port, pid);
  Command::new("/bin/bash") // hmm.... not best
    .arg("-c")
    .arg(format!("adb forward tcp:{} jdwp:{}", tcp_port, pid))
    .output()
    .expect("failed to execute process");
}