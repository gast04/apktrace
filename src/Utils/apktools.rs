use std::process::Command;

#[allow(dead_code)]
pub fn start_application(package_id: &String, activity: &String) -> Result<bool, String> {
    println!("starting application");

    let mut start_str: String = String::from(package_id);
    start_str.push('/');
    start_str.push_str(activity);

    Command::new("adb")
        .arg("shell")
        .arg("am start -D -n")
        .arg(start_str)
        .output()
        .expect("failed to execute process");

    return Err("Hell no".to_string());
}

pub fn get_pid_by_package(package_name: &str) -> Result<u64, String> {
    let output = Command::new("adb")
        .arg("shell")
        .arg("pidof")
        .arg(package_name)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout.is_empty() {
                return Err(format!("No process found for package: {}", package_name));
            }
            let pid_str = stdout.split_whitespace().next().unwrap_or("");
            match pid_str.parse::<u64>() {
                Ok(pid) => Ok(pid),
                Err(_) => Err(format!("Could not parse PID from: {}", stdout)),
            }
        }
        Err(e) => Err(format!("Failed to run adb: {}", e)),
    }
}

pub fn list_debuggable_pids() -> Vec<(u64, String)> {
    use std::io::Read;
    use std::process::Stdio;

    let mut results: Vec<(u64, String)> = Vec::new();

    let child = Command::new("adb")
        .arg("jdwp")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    if let Ok(mut child) = child {
        std::thread::sleep(std::time::Duration::from_millis(300));

        let _ = child.kill();

        if let Some(mut stdout) = child.stdout.take() {
            let mut output = String::new();
            let _ = stdout.read_to_string(&mut output);

            for line in output.lines() {
                if let Ok(pid) = line.trim().parse::<u64>() {
                    let name = get_package_by_pid(pid).unwrap_or_else(|| format!("pid:{}", pid));
                    results.push((pid, name));
                }
            }
        }
    }

    results
}

pub fn get_package_by_pid(pid: u64) -> Option<String> {
    let output = Command::new("adb")
        .arg("shell")
        .arg(format!("cat /proc/{}/cmdline", pid))
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let name = stdout.trim_matches('\0').trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

pub fn forward_jdwp(tcp_port: u64, pid: u64) {
    println!("[apktrace] adb forward tcp:{} jdwp:{}", tcp_port, pid);
    Command::new("/bin/bash")
        .arg("-c")
        .arg(format!("adb forward tcp:{} jdwp:{}", tcp_port, pid))
        .output()
        .expect("failed to execute process");
}
