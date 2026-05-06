use colored::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

#[derive(Clone)]
pub struct MethodCall {
    pub class_id: u64,
    pub method_id: u64,
    pub entry_time: Instant,
    pub child_time_us: u64,
}

pub struct MethodStats {
    pub class_name: String,
    pub method_name: String,
    pub call_count: u64,
    pub total_time_us: u64,
    pub self_time_us: u64,
    pub min_time_us: u64,
    pub max_time_us: u64,
}

impl MethodStats {
    pub fn new(class_name: String, method_name: String) -> Self {
        MethodStats {
            class_name,
            method_name,
            call_count: 0,
            total_time_us: 0,
            self_time_us: 0,
            min_time_us: u64::MAX,
            max_time_us: 0,
        }
    }

    pub fn record(&mut self, duration_us: u64, self_duration_us: u64) {
        self.call_count += 1;
        self.total_time_us += duration_us;
        self.self_time_us += self_duration_us;
        if duration_us < self.min_time_us {
            self.min_time_us = duration_us;
        }
        if duration_us > self.max_time_us {
            self.max_time_us = duration_us;
        }
    }

    pub fn avg_time_us(&self) -> u64 {
        if self.call_count == 0 {
            return 0;
        }
        self.total_time_us / self.call_count
    }
}

pub struct Tracer {
    call_stacks: HashMap<u64, Vec<MethodCall>>,
    method_stats: HashMap<(u64, u64), MethodStats>,
    pub total_events: u64,
    log_writer: Option<BufWriter<File>>,
    backtrace_writer: Option<BufWriter<File>>,
    seen_backtraces: HashSet<Vec<(u64, u64)>>,
    backtrace_total: u64,
    backtrace_unique: u64,
}

impl Tracer {
    pub fn new() -> Self {
        Tracer {
            call_stacks: HashMap::new(),
            method_stats: HashMap::new(),
            total_events: 0,
            log_writer: None,
            backtrace_writer: None,
            seen_backtraces: HashSet::new(),
            backtrace_total: 0,
            backtrace_unique: 0,
        }
    }

    pub fn set_log_file(&mut self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        self.log_writer = Some(BufWriter::new(file));
        Ok(())
    }

    pub fn set_backtrace_file(&mut self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        self.backtrace_writer = Some(BufWriter::new(file));
        Ok(())
    }

    pub fn has_backtrace_log(&self) -> bool {
        self.backtrace_writer.is_some()
    }

    pub fn log(&mut self, line: &str) {
        if let Some(ref mut writer) = self.log_writer {
            let _ = writeln!(writer, "{}", line);
        } else {
            println!("{}", line);
        }
    }

    pub fn is_backtrace_new(&mut self, signature: Vec<(u64, u64)>) -> bool {
        self.backtrace_total += 1;
        if self.seen_backtraces.contains(&signature) {
            return false;
        }
        self.seen_backtraces.insert(signature);
        self.backtrace_unique += 1;
        true
    }

    pub fn log_backtrace(&mut self, lines: &[String]) {
        if let Some(ref mut writer) = self.backtrace_writer {
            for line in lines {
                let _ = writeln!(writer, "{}", line);
            }
            let _ = writeln!(writer);
        }
    }

    pub fn flush(&mut self) {
        if let Some(ref mut writer) = self.log_writer {
            let _ = writer.flush();
        }
        if let Some(ref mut writer) = self.backtrace_writer {
            let _ = writer.flush();
        }
    }

    pub fn method_entry(&mut self, thread_id: u64, class_id: u64, method_id: u64) -> usize {
        self.total_events += 1;

        let call = MethodCall {
            class_id,
            method_id,
            entry_time: Instant::now(),
            child_time_us: 0,
        };

        let stack = self.call_stacks.entry(thread_id).or_insert_with(Vec::new);
        stack.push(call);
        stack.len()
    }

    pub fn method_exit(
        &mut self,
        thread_id: u64,
        class_id: u64,
        method_id: u64,
        class_name: &str,
        method_name: &str,
    ) -> Option<u64> {
        self.total_events += 1;

        let stack = self.call_stacks.get_mut(&thread_id)?;

        let mut found_idx: Option<usize> = None;
        for (i, call) in stack.iter().enumerate().rev() {
            if call.class_id == class_id && call.method_id == method_id {
                found_idx = Some(i);
                break;
            }
        }

        let idx = found_idx?;
        let call = stack.remove(idx);
        let duration_us = call.entry_time.elapsed().as_micros() as u64;
        let self_time_us = duration_us.saturating_sub(call.child_time_us);

        if let Some(parent) = stack.last_mut() {
            parent.child_time_us += duration_us;
        }

        let key = (class_id, method_id);
        let stats = self
            .method_stats
            .entry(key)
            .or_insert_with(|| MethodStats::new(class_name.to_string(), method_name.to_string()));
        stats.record(duration_us, self_time_us);

        Some(duration_us)
    }

    pub fn get_stack_depth(&self, thread_id: u64) -> usize {
        self.call_stacks.get(&thread_id).map_or(0, |s| s.len())
    }

    pub fn print_summary(&self) {
        println!("\n{}", "═".repeat(110).cyan());
        println!("{}", " Method Timing Summary ".cyan().bold());
        println!("{}", "═".repeat(110).cyan());

        println!("\nTotal events processed: {}", self.total_events);
        println!("Unique methods traced: {}", self.method_stats.len());
        if self.backtrace_total > 0 {
            println!(
                "Backtraces: {} unique / {} total",
                self.backtrace_unique, self.backtrace_total
            );
        }
        println!();

        if self.method_stats.is_empty() {
            println!("No method timing data collected.");
            return;
        }

        let mut stats_vec: Vec<&MethodStats> = self.method_stats.values().collect();

        let self_total: u64 = stats_vec.iter().map(|s| s.self_time_us).sum();
        let self_total_f = self_total as f64;

        stats_vec.sort_by(|a, b| b.self_time_us.cmp(&a.self_time_us));

        println!(
            "{:>7} {:>10} {:>10} {:>10} {:>10} {:>10}  {}",
            "Self%".bold(),
            "Calls".bold(),
            "Self(ms)".bold(),
            "Total(ms)".bold(),
            "Avg(us)".bold(),
            "Max(us)".bold(),
            "Method".bold()
        );
        println!("{}", "─".repeat(110));

        let top_n = std::cmp::min(30, stats_vec.len());
        for stats in stats_vec.iter().take(top_n) {
            let self_ms = stats.self_time_us as f64 / 1000.0;
            let total_ms = stats.total_time_us as f64 / 1000.0;
            let self_pct = if self_total > 0 {
                (stats.self_time_us as f64 / self_total_f) * 100.0
            } else {
                0.0
            };

            println!(
                "{:>6.1}% {:>10} {:>10.2} {:>10.2} {:>10} {:>10}  {} -> {}",
                self_pct,
                stats.call_count,
                self_ms,
                total_ms,
                stats.avg_time_us(),
                stats.max_time_us,
                stats.class_name.yellow(),
                stats.method_name
            );
        }

        if stats_vec.len() > top_n {
            println!("\n... and {} more methods", stats_vec.len() - top_n);
        }

        println!(
            "\n{}: time in function only (excludes callees)",
            "Self%".bold()
        );
        println!("{}: time including all nested calls", "Total".bold());
        println!("{}", "═".repeat(110).cyan());
    }
}

pub fn format_duration(us: u64) -> String {
    if us < 1000 {
        format!("{}us", us)
    } else if us < 1_000_000 {
        format!("{:.2}ms", us as f64 / 1000.0)
    } else {
        format!("{:.2}s", us as f64 / 1_000_000.0)
    }
}
