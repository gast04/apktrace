use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use argparse::{ArgumentParser, Print, Store, StoreTrue};

#[path = "Utils/apktools.rs"]
mod apktools;
#[path = "JdwpHandler/jdwp_handler.rs"]
mod jdwp_handler;

fn main() {
    println!("apktrace - JDWP Method Tracer & Performance Analyzer");
    println!("Press Ctrl+C to stop tracing and show summary\n");

    let mut target: String = "".to_string();
    let mut class_pattern: String = "".to_string();
    let mut verbose: bool = false;
    let mut tcp_port: u64 = 33333;
    let mut list_processes: bool = false;
    let mut output_file: String = "".to_string();
    let mut backtrace_file: String = "".to_string();

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Attach to a running Android/Java application via JDWP to trace method entry/exit with timing");

        ap.refer(&mut target).add_argument(
            "target",
            Store,
            "Process PID or package name",
        );

        ap.refer(&mut class_pattern).add_option(
            &["-c", "--class"],
            Store,
            "Class pattern to trace (e.g., 'com.myapp.*')",
        );

        ap.refer(&mut tcp_port).add_option(
            &["-p", "--port"],
            Store,
            "Local TCP port for JDWP forwarding (default: 33333)",
        );

        ap.refer(&mut output_file).add_option(
            &["-o", "--output"],
            Store,
            "Output file for trace log (default: stdout)",
        );

        ap.refer(&mut backtrace_file).add_option(
            &["-b", "--backtrace"],
            Store,
            "Output file for backtraces on METHOD_ENTRY events",
        );

        ap.refer(&mut verbose)
            .add_option(&["--verbose"], StoreTrue, "Enable verbose output");

        ap.refer(&mut list_processes).add_option(
            &["-l", "--list"],
            StoreTrue,
            "List debuggable processes",
        );

        ap.add_option(
            &["-v", "--version"],
            Print("apktrace version 4.0.0".to_string()),
            "Show version",
        );

        ap.parse_args_or_exit();
    }

    if list_processes {
        println!("Debuggable processes:");
        let procs = apktools::list_debuggable_pids();
        if procs.is_empty() {
            println!("  No debuggable processes found. Is USB debugging enabled?");
        } else {
            for (pid, name) in procs {
                println!("  {:>8}  {}", pid, name);
            }
        }
        std::process::exit(0);
    }

    if target.is_empty() {
        println!("Error: No target given!");
        println!("Usage: apktrace <pid|package> [-c class_pattern] [-p port]");
        println!("       apktrace -l    # list debuggable processes");
        std::process::exit(-1);
    }

    let target_pid: u64 = match target.parse::<u64>() {
        Ok(pid) => pid,
        Err(_) => {
            println!("Looking up PID for package: {}", target);
            match apktools::get_pid_by_package(&target) {
                Ok(pid) => {
                    println!("Found PID: {}", pid);
                    pid
                }
                Err(e) => {
                    println!("Error: {}", e);
                    println!("Make sure the app is running in debug mode.");
                    std::process::exit(-1);
                }
            }
        }
    };

    apktools::forward_jdwp(tcp_port, target_pid);

    let res = jdwp_handler::init_connection("127.0.0.1", tcp_port as u16, verbose);
    if !res.is_ok() {
        println!("Failed to connect to JDWP. Is the app running in debug mode?");
        process::exit(-1);
    }
    let mut handler = res.unwrap();

    if !output_file.is_empty() {
        match jdwp_handler::set_log_file(&mut handler, &output_file) {
            Ok(_) => println!("[apktrace] Logging to: {}", output_file),
            Err(e) => {
                println!("Error: Could not open output file: {}", e);
                process::exit(-1);
            }
        }
    }

    if !backtrace_file.is_empty() {
        match jdwp_handler::set_backtrace_file(&mut handler, &backtrace_file) {
            Ok(_) => println!("[apktrace] Backtraces to: {}", backtrace_file),
            Err(e) => {
                println!("Error: Could not open backtrace file: {}", e);
                process::exit(-1);
            }
        }
    }

    if class_pattern.is_empty() {
        if let Err(e) = jdwp_handler::break_on_method_entry(&mut handler, "") {
            println!("Failed to register METHOD_ENTRY event: {}", e);
            process::exit(-1);
        }
        if let Err(e) = jdwp_handler::break_on_method_exit_wrv(&mut handler, "") {
            println!("Failed to register METHOD_EXIT event: {}", e);
            process::exit(-1);
        }
    } else {
        println!("[apktrace] Tracing classes matching: {}", class_pattern);
        if let Err(e) = jdwp_handler::break_on_method_entry_match(&mut handler, &class_pattern) {
            println!("Failed to register METHOD_ENTRY event: {}", e);
            process::exit(-1);
        }
        if let Err(e) = jdwp_handler::break_on_method_exit_match(&mut handler, &class_pattern) {
            println!("Failed to register METHOD_EXIT event: {}", e);
            process::exit(-1);
        }
    }

    if let Err(e) = jdwp_handler::resume_vm(&mut handler) {
        println!("Failed to resume VM: {}", e);
        process::exit(-1);
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc_handler(move || {
        r.store(false, Ordering::SeqCst);
    });

    println!("Tracing started. Waiting for method events...\n");

    let mut event_count: u64 = 0;
    let start_time = std::time::Instant::now();
    let mut last_report = start_time;
    let mut consecutive_failures = 0;
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;

    while running.load(Ordering::SeqCst) {
        match jdwp_handler::wait_for_event(&mut handler) {
            Ok(processed_events) => {
                consecutive_failures = 0;
                event_count += processed_events as u64;
                if let Err(e) = jdwp_handler::resume_vm(&mut handler) {
                    eprintln!("[apktrace] Failed to resume VM: {}", e);
                    break;
                }
            }
            Err(e) => {
                consecutive_failures += 1;
                eprintln!(
                    "[apktrace] Failed to read/process JDWP event ({}/{}): {}",
                    consecutive_failures, MAX_CONSECUTIVE_FAILURES, e
                );
                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    eprintln!("[apktrace] Too many JDWP communication failures; stopping trace.");
                    break;
                }
                continue;
            }
        }

        let now = std::time::Instant::now();
        if now.duration_since(last_report).as_secs() >= 5 {
            let elapsed = now.duration_since(start_time).as_secs();
            let rate = if elapsed > 0 {
                event_count / elapsed
            } else {
                0
            };
            eprintln!("[apktrace] {} events ({}/sec)", event_count, rate);
            last_report = now;
        }
    }

    jdwp_handler::flush_log(&mut handler);
    println!("\nTracing stopped.");
    jdwp_handler::print_summary(&handler);
}

fn ctrlc_handler<F>(handler: F)
where
    F: FnOnce() + Send + 'static,
{
    let handler = std::sync::Mutex::new(Some(handler));

    ctrlc::set_handler(move || {
        if let Some(h) = handler.lock().unwrap().take() {
            h();
        }
    })
    .expect("Error setting Ctrl-C handler");
}
