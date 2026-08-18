use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

const A64DBG_DEVICE_PATH: &str = "/data/local/tmp/a64dbg";

pub struct NativeLibTracer {
    #[allow(dead_code)]
    a64dbg_child: Option<Child>,
    tracer_thread: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
    lib_count: Arc<std::sync::atomic::AtomicU64>,
    log_path: String,
}

impl NativeLibTracer {
    pub fn new(pid: u64, log_path: &str, verbose: bool) -> Result<Self, String> {
        Self::ensure_a64dbg_on_device(verbose)?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let lib_count = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let stop_flag_clone = stop_flag.clone();
        let lib_count_clone = lib_count.clone();
        let log_path_owned = log_path.to_string();
        let log_path_for_thread = log_path.to_string();

        let tracer_thread = thread::spawn(move || {
            if let Err(e) = Self::run_a64dbg(
                pid,
                &log_path_for_thread,
                stop_flag_clone,
                lib_count_clone,
                verbose,
            ) {
                eprintln!("[libtracer] a64dbg error: {}", e);
            }
        });

        Ok(NativeLibTracer {
            a64dbg_child: None,
            tracer_thread: Some(tracer_thread),
            stop_flag,
            lib_count,
            log_path: log_path_owned,
        })
    }

    fn ensure_a64dbg_on_device(verbose: bool) -> Result<(), String> {
        let check = Command::new("adb")
            .args([
                "shell",
                &format!("test -f {} && echo exists", A64DBG_DEVICE_PATH),
            ])
            .output()
            .map_err(|e| format!("Failed to check a64dbg: {}", e))?;

        let output = String::from_utf8_lossy(&check.stdout);
        if output.trim() == "exists" {
            if verbose {
                println!("[libtracer] a64dbg already on device");
            }
            return Ok(());
        }

        let a64dbg_local = std::env::var("A64DBG_PATH").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            format!("{}/gitrepos/a64dbg/build/a64dbg", home)
        });

        if !std::path::Path::new(&a64dbg_local).exists() {
            return Err(format!(
                "a64dbg not found at {}. Set A64DBG_PATH or build a64dbg first.",
                a64dbg_local
            ));
        }

        if verbose {
            println!("[libtracer] Pushing a64dbg to device...");
        }

        let push = Command::new("adb")
            .args(["push", &a64dbg_local, A64DBG_DEVICE_PATH])
            .output()
            .map_err(|e| format!("Failed to push a64dbg: {}", e))?;

        if !push.status.success() {
            return Err(format!(
                "Failed to push a64dbg: {}",
                String::from_utf8_lossy(&push.stderr)
            ));
        }

        Command::new("adb")
            .args(["shell", "chmod", "755", A64DBG_DEVICE_PATH])
            .output()
            .ok();

        Ok(())
    }

    fn run_a64dbg(
        pid: u64,
        log_path: &str,
        stop_flag: Arc<AtomicBool>,
        lib_count: Arc<std::sync::atomic::AtomicU64>,
        verbose: bool,
    ) -> Result<(), String> {
        let device_log = format!("/data/local/tmp/libload_{}.log", pid);

        let cmd = format!("{} -a {} --libload {}", A64DBG_DEVICE_PATH, pid, device_log);

        if verbose {
            println!("[libtracer] Running: adb shell {}", cmd);
        }

        let mut child = Command::new("adb")
            .args(["shell", &cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn a64dbg: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            if stop_flag.load(Ordering::SeqCst) {
                break;
            }

            if let Ok(line) = line {
                if verbose || line.contains("LOAD:") {
                    println!("{}", line);
                }

                if line.contains("LOAD:") {
                    lib_count.fetch_add(1, Ordering::SeqCst);
                }

                if line.contains("Process exited") || line.contains("terminated") {
                    break;
                }
            }
        }

        let _ = child.kill();
        let _ = child.wait();

        let pull = Command::new("adb")
            .args(["pull", &device_log, log_path])
            .output();

        if let Ok(out) = pull {
            if !out.status.success() && verbose {
                eprintln!("[libtracer] Could not pull log file from device");
            }
        }

        Command::new("adb")
            .args(["shell", "rm", "-f", &device_log])
            .output()
            .ok();

        Ok(())
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);

        Command::new("adb")
            .args(["shell", "pkill", "-2", "a64dbg"])
            .output()
            .ok();

        if let Some(handle) = self.tracer_thread.take() {
            let _ = handle.join();
        }
    }

    pub fn print_summary(&self) {
        let count = self.lib_count.load(Ordering::SeqCst);
        println!("\n[libtracer] Native libraries loaded: {}", count);
        println!("[libtracer] Log written to: {}", self.log_path);
    }
}

impl Drop for NativeLibTracer {
    fn drop(&mut self) {
        self.stop();
    }
}
