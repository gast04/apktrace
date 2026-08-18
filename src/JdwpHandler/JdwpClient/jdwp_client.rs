#[path = "classes.rs"]
mod class;
#[path = "Connection/connection.rs"]
mod connection;
#[path = "events.rs"]
mod event;
#[path = "methods.rs"]
mod method;
#[path = "protocol_vars.rs"]
pub(super) mod pvars;
#[path = "tracer.rs"]
pub mod tracer;
#[path = "utils.rs"]
mod utils;

use connection::Connection as Conn;

use colored::*;

pub struct JdwpClient {
    pub con: Conn,
    pub idsizes: utils::IdSizes,
    pub classes: class::Classes,
    pub methods: method::Methods,
    pub threads: utils::Threads,
    pub created_events: Vec<(u8, u32)>, // (Event kind, request id)
    pub version: utils::Version,
    pub tracer: tracer::Tracer,
    verbose: bool,
}

// add functions to the struct
impl JdwpClient {
    pub fn new(host: &str, port: u16, verbose: bool) -> Result<Self, String> {
        let con = Conn::new(host, port, verbose).map_err(|e| {
            println!("[JdpwClient] Could not connect to server: {}", e);
            e
        })?;

        Ok(JdwpClient {
            con,
            idsizes: utils::IdSizes::new(),
            classes: class::Classes::new(),
            methods: method::Methods::new(),
            threads: utils::Threads::new(),
            created_events: Vec::new(),
            version: utils::Version::new(),
            tracer: tracer::Tracer::new(),
            verbose,
        })
    }

    fn dbg_print(&self, msg: &str) {
        if self.verbose {
            println!("[JdwpClient] {}", msg);
        }
    }

    pub fn get_version(&mut self) -> Result<(), String> {
        self.dbg_print("get_version");
        let packet_id = self.con.send_packet(pvars::VERSION_SIG, b"")?;
        let data_buffer = self.con.read_reply_buffer(packet_id)?;
        self.version = utils::parse_version(&data_buffer)?;
        if self.verbose {
            self.version.print();
        }
        Ok(())
    }

    pub fn print_version(&mut self) {
        let parts = self.version.description.split("\n");
        for s in parts {
            println!("{} {}", "[JdwpClient]".yellow(), s);
        }
    }

    pub fn handshake(&mut self) -> Result<(), String> {
        self.dbg_print("handshake");

        let handshake_str = b"JDWP-Handshake";
        self.con.send_raw(handshake_str)?;

        let mut buffer: [u8; 14] = [0; 14];
        self.con.read_raw(&mut buffer)?;

        if !buffer.eq(handshake_str) {
            self.dbg_print("Handshake failed");
            return Err("Handshake response did not match JDWP-Handshake".to_string());
        }

        Ok(())
    }

    pub fn suspend_vm(&mut self) -> Result<(), String> {
        self.dbg_print("suspend_vm");
        let packet_id = self.con.send_packet(pvars::SUSPENDVM_SIG, b"")?;
        self.con.wait_reply(packet_id)
    }

    pub fn resume_vm(&mut self) -> Result<(), String> {
        self.dbg_print("resume_vm");
        let packet_id = self.con.send_packet(pvars::RESUMEVM_SIG, b"")?;
        self.con.wait_reply(packet_id)
    }

    pub fn get_idsizes(&mut self) -> Result<(), String> {
        self.dbg_print("get_idsizes");
        let packet_id = self.con.send_packet(pvars::IDSIZES_SIG, b"")?;
        let data_buffer = self.con.read_reply_buffer(packet_id)?;
        self.idsizes = utils::parse_idsizes(&data_buffer)?;
        if self.verbose {
            self.idsizes.print();
        }
        Ok(())
    }

    pub fn fetch_classes(&mut self) -> Result<(), String> {
        self.dbg_print("fetch_classes");

        class::fetch_all_classes(
            &mut self.con,
            &mut self.classes,
            self.idsizes.reference_type_id_size,
        )?;

        self.dbg_print(&format!("Fetched classes: {}", self.classes.vec.len()));
        Ok(())
    }

    fn class_is_default_traced(signature: &str) -> bool {
        !signature.starts_with("Ljava/")
            && !signature.starts_with("Ljavax/")
            && !signature.starts_with("Lsun/")
            && !signature.starts_with("Lcom/sun/")
            && !signature.starts_with("Ldalvik/system/")
            && !signature.starts_with("Llibcore/")
            && !signature.starts_with("Landroid/")
            && !signature.starts_with("Lcom/android/")
            && !signature.starts_with("Landroidx/")
    }

    fn wildcard_match(value: &str, pattern: &str) -> bool {
        let mut rest = value;
        let mut first = true;
        for part in pattern.split('*') {
            if part.is_empty() {
                continue;
            }

            if first && !pattern.starts_with('*') {
                let Some(stripped) = rest.strip_prefix(part) else {
                    return false;
                };
                rest = stripped;
            } else {
                let Some(idx) = rest.find(part) else {
                    return false;
                };
                rest = &rest[idx + part.len()..];
            }
            first = false;
        }

        pattern.ends_with('*') || rest.is_empty()
    }

    fn class_matches_pattern(signature: &str, pattern: &str) -> bool {
        if pattern.is_empty() {
            return Self::class_is_default_traced(signature);
        }

        if Self::wildcard_match(signature, pattern) {
            return true;
        }

        let slash_pattern = pattern.replace('.', "/");
        if Self::wildcard_match(signature, &slash_pattern) {
            return true;
        }

        let unwrapped_signature = signature
            .strip_prefix('L')
            .unwrap_or(signature)
            .strip_suffix(';')
            .unwrap_or(signature);
        let dotted_signature = unwrapped_signature.replace('/', ".");
        Self::wildcard_match(&dotted_signature, pattern)
    }

    pub fn warm_log_only_caches(&mut self, class_pattern: &str) -> Result<(), String> {
        println!("[apktrace] Preloading thread names for log-only mode...");
        match utils::get_all_thread_ids(&mut self.con, self.idsizes.object_id_size) {
            Ok(thread_ids) => {
                for thread_id in thread_ids {
                    if self.threads.get(thread_id).is_some() {
                        continue;
                    }
                    match utils::get_thread_by_id(
                        &mut self.con,
                        self.idsizes.object_id_size,
                        thread_id,
                    ) {
                        Ok(name) => {
                            self.threads.insert(thread_id, name);
                        }
                        Err(e) => {
                            eprintln!(
                                "[apktrace] Warning: could not preload thread {}: {}",
                                thread_id, e
                            );
                        }
                    }
                }
                println!("[apktrace] Preloaded {} thread names", self.threads.len());
            }
            Err(e) => {
                eprintln!("[apktrace] Warning: could not list threads: {}", e);
            }
        }

        let class_ids: Vec<(u64, String)> = self
            .classes
            .vec
            .iter()
            .filter(|class| Self::class_matches_pattern(&class.signature, class_pattern))
            .map(|class| (class.ref_type_id, class.signature.clone()))
            .collect();

        println!(
            "[apktrace] Preloading methods for {} traced classes...",
            class_ids.len()
        );

        let mut loaded_classes = 0usize;
        let mut loaded_methods = 0usize;
        let mut failed_classes = 0usize;

        for (idx, (class_id, signature)) in class_ids.iter().enumerate() {
            match method::fetch_methods_for_class(
                &mut self.con,
                &mut self.methods,
                self.idsizes.method_id_size,
                self.idsizes.object_id_size,
                *class_id,
            ) {
                Ok(count) => {
                    loaded_classes += 1;
                    loaded_methods += count;
                }
                Err(e) => {
                    failed_classes += 1;
                    if self.verbose {
                        eprintln!(
                            "[apktrace] Warning: could not preload methods for {}: {}",
                            signature, e
                        );
                    }
                }
            }

            if self.verbose && (idx + 1) % 250 == 0 {
                eprintln!(
                    "[apktrace] Preloaded methods for {}/{} classes...",
                    idx + 1,
                    class_ids.len()
                );
            }
        }

        println!(
            "[apktrace] Preloaded {} methods from {} classes ({} failed)",
            loaded_methods, loaded_classes, failed_classes
        );
        Ok(())
    }

    fn register_method_event(
        &mut self,
        event_kind: u8,
        class_pattern: Option<&str>,
        suspend_policy: u8,
    ) -> Result<(), String> {
        let request_id = match class_pattern {
            Some(pattern) => {
                event::class_match_event(&mut self.con, pattern, event_kind, suspend_policy)?
            }
            None => event::class_exclude_event(&mut self.con, event_kind, suspend_policy)?,
        };
        self.created_events.push((event_kind, request_id));
        Ok(())
    }

    pub fn evt_entry_class_match(
        &mut self,
        class_pattern: &str,
        suspend_policy: u8,
    ) -> Result<(), String> {
        self.dbg_print("evt_entry_class_match");
        self.register_method_event(
            pvars::EVENT_METHOD_ENTRY,
            Some(class_pattern),
            suspend_policy,
        )
    }

    pub fn evt_entry_class_exclude(&mut self, suspend_policy: u8) -> Result<(), String> {
        self.dbg_print("evt_entry_class_exclude");
        self.register_method_event(pvars::EVENT_METHOD_ENTRY, None, suspend_policy)
    }

    pub fn evt_exit_wrv_class_exclude(&mut self, suspend_policy: u8) -> Result<(), String> {
        self.dbg_print("evt_exit_wrv_class_exclude");
        self.register_method_event(pvars::EVENT_METHOD_EXIT_WRV, None, suspend_policy)
    }

    pub fn evt_exit_wrv_class_match(
        &mut self,
        class_pattern: &str,
        suspend_policy: u8,
    ) -> Result<(), String> {
        self.dbg_print("evt_exit_wrv_class_match");
        self.register_method_event(
            pvars::EVENT_METHOD_EXIT_WRV,
            Some(class_pattern),
            suspend_policy,
        )
    }

    pub fn wait_for_event(&mut self, allow_jdwp_lookups: bool) -> Result<usize, String> {
        let buffer = self.con.read_buffer()?;

        let response = event::parse_event_response(
            &buffer,
            self.idsizes.object_id_size,
            self.idsizes.reference_type_id_size,
            self.idsizes.method_id_size,
        );
        if self.verbose {
            println!(
                "[JdwpClient] Received {} bytes, {} events",
                buffer.len(),
                response.events.len()
            );
            response.print();
        }

        if response.events.is_empty() {
            return Ok(0);
        }

        let mut handled_events = 0;
        for event in &response.events {
            let matching_kind = self
                .created_events
                .iter()
                .find(|(_, r_id)| event.request_id == *r_id)
                .map(|(kind, _)| *kind);

            let Some(kind) = matching_kind else {
                continue;
            };

            let class_name = if allow_jdwp_lookups {
                class::get_name_by_id(
                    &mut self.con,
                    &mut self.classes,
                    self.idsizes.reference_type_id_size,
                    event.class_id,
                )?
            } else {
                self.classes
                    .get_cached_name_by_id(event.class_id)
                    .unwrap_or_else(|| format!("Lunknown/{};", event.class_id))
            };

            let method = if allow_jdwp_lookups {
                method::get_method_by_id(
                    &mut self.con,
                    &mut self.methods,
                    self.idsizes.method_id_size,
                    self.idsizes.object_id_size,
                    event.class_id,
                    event.method_id,
                )?
            } else {
                self.methods
                    .get_cached_by_id(event.class_id, event.method_id)
                    .unwrap_or_else(|| method::Method {
                        ref_type_id: event.class_id,
                        method_id: event.method_id,
                        name: format!("<method:{}>", event.method_id),
                        signature: String::new(),
                        modbits: 0,
                        ret_void: true,
                        native: false,
                    })
            };

            let method_display = format!("{}{}", method.name, method.signature);

            let thread_name = if allow_jdwp_lookups {
                match self.threads.get(event.thread_id) {
                    Some(info) => info.name.clone(),
                    None => {
                        let name = utils::get_thread_by_id(
                            &mut self.con,
                            self.idsizes.object_id_size,
                            event.thread_id,
                        )?;
                        self.threads.insert(event.thread_id, name).name.clone()
                    }
                }
            } else {
                self.threads
                    .get(event.thread_id)
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| "thread".to_string())
            };

            let time = utils::get_current_time();
            let native_flag = if method.native { "N" } else { " " };

            if kind == pvars::EVENT_METHOD_ENTRY {
                let depth =
                    self.tracer
                        .method_entry(event.thread_id, event.class_id, event.method_id);

                let indent = "  ".repeat(depth.saturating_sub(1));
                let line = format!(
                    "[{}] >> [{}:{}] {} {}{} -> {}",
                    time,
                    thread_name,
                    event.thread_id,
                    native_flag,
                    indent,
                    class_name,
                    method_display
                );
                self.tracer.log(&line);

                if self.tracer.has_backtrace_log() {
                    if let Ok(frames) = utils::get_thread_frames(
                        &mut self.con,
                        self.idsizes.object_id_size,
                        self.idsizes.reference_type_id_size,
                        self.idsizes.method_id_size,
                        self.idsizes.frame_id_size,
                        event.thread_id,
                        -1,
                    ) {
                        let signature: Vec<(u64, u64)> =
                            frames.iter().map(|f| (f.class_id, f.method_id)).collect();

                        if self.tracer.is_backtrace_new(signature) {
                            let mut bt_lines: Vec<String> = Vec::new();
                            bt_lines.push(format!(
                                "{} -> {} (thread: {})",
                                class_name, method_display, thread_name
                            ));

                            for (i, frame) in frames.iter().enumerate() {
                                let frame_class = class::get_name_by_id(
                                    &mut self.con,
                                    &mut self.classes,
                                    self.idsizes.reference_type_id_size,
                                    frame.class_id,
                                )
                                .unwrap_or_else(|_| format!("<class:{}>", frame.class_id));

                                let frame_method = method::get_method_by_id(
                                    &mut self.con,
                                    &mut self.methods,
                                    self.idsizes.method_id_size,
                                    self.idsizes.object_id_size,
                                    frame.class_id,
                                    frame.method_id,
                                )
                                .map(|m| format!("{}{}", m.name, m.signature))
                                .unwrap_or_else(|_| format!("<method:{}>", frame.method_id));

                                bt_lines
                                    .push(format!("  #{} {} -> {}", i, frame_class, frame_method));
                            }
                            self.tracer.log_backtrace(&bt_lines);
                        }
                    }
                }
            } else if kind == pvars::EVENT_METHOD_EXIT_WRV {
                let duration_us = self.tracer.method_exit(
                    event.thread_id,
                    event.class_id,
                    event.method_id,
                    &class_name,
                    &method_display,
                );

                let depth = self.tracer.get_stack_depth(event.thread_id);
                let indent = "  ".repeat(depth);

                let duration_str = match duration_us {
                    Some(us) => format!(" {}", tracer::format_duration(us)),
                    None => String::new(),
                };

                let retval_str = if !method.ret_void {
                    format!(" = {}", event.retval)
                } else {
                    String::new()
                };

                let line = format!(
                    "[{}] << [{}:{}] {} {}{} -> {}{}{}",
                    time,
                    thread_name,
                    event.thread_id,
                    native_flag,
                    indent,
                    class_name,
                    method_display,
                    duration_str,
                    retval_str
                );
                self.tracer.log(&line);
            }
            handled_events += 1;
        }

        Ok(handled_events)
    }

    pub fn set_log_file(&mut self, path: &str) -> std::io::Result<()> {
        self.tracer.set_log_file(path)
    }

    pub fn set_backtrace_file(&mut self, path: &str) -> std::io::Result<()> {
        self.tracer.set_backtrace_file(path)
    }

    pub fn flush_log(&mut self) {
        self.tracer.flush();
    }

    pub fn print_trace_summary(&self) {
        self.tracer.print_summary();
    }
}
