#[path = "JdwpClient/jdwp_client.rs"]
mod jdwp_client;

pub struct JdwpHandler {
    client: jdwp_client::JdwpClient,
    verbose: bool,
}

impl JdwpHandler {
    pub fn new(host: &str, port: u16, verbose: bool) -> Result<Self, String> {
        let res = jdwp_client::JdwpClient::new(host, port, verbose);
        if let Err(e) = res {
            println!("[JdwpHandler] Could not create Client: {}", e);
            return Err(e);
        }
        let client = res.unwrap();
        return Ok(JdwpHandler {
            client: client,
            verbose: verbose,
        });
    }
}

pub fn init_connection(host: &str, port: u16, verbose: bool) -> Result<JdwpHandler, String> {
    let res = JdwpHandler::new(host, port, verbose);
    if res.is_err() {
        println!(
            "[apktrace] Failed to create TCP connection to {}:{}",
            host, port
        );
        return res;
    }
    let mut _handler = res.unwrap();
    println!("[apktrace] Connected to JDWP");

    if let Err(e) = _handler.client.handshake() {
        println!("[apktrace] JDWP handshake failed: {}", e);
        return Err(e);
    }
    println!("[apktrace] Handshake successful");

    _handler.client.suspend_vm()?;
    _handler.client.get_idsizes()?;
    _handler.client.get_version()?;
    _handler.client.fetch_classes()?;

    _handler.client.print_version();
    println!(
        "[apktrace] Loaded {} classes",
        _handler.client.classes.vec.len()
    );

    return Ok(_handler);
}

pub fn break_on_method_entry(
    handler: &mut JdwpHandler,
    _class_pattern: &str,
) -> Result<(), String> {
    handler.client.evt_entry_class_exclude()?;
    println!("[apktrace] Registered METHOD_ENTRY event (exclude mode)");
    Ok(())
}

pub fn break_on_method_exit_wrv(
    handler: &mut JdwpHandler,
    _class_pattern: &str,
) -> Result<(), String> {
    handler.client.evt_exit_wrv_class_exclude()?;
    println!("[apktrace] Registered METHOD_EXIT event (exclude mode)");
    Ok(())
}

pub fn break_on_method_entry_match(
    handler: &mut JdwpHandler,
    class_pattern: &str,
) -> Result<(), String> {
    handler.client.evt_entry_class_match(class_pattern)?;
    println!(
        "[apktrace] Registered METHOD_ENTRY event (match: {})",
        class_pattern
    );
    Ok(())
}

pub fn break_on_method_exit_match(
    handler: &mut JdwpHandler,
    class_pattern: &str,
) -> Result<(), String> {
    handler.client.evt_exit_wrv_class_match(class_pattern)?;
    println!(
        "[apktrace] Registered METHOD_EXIT event (match: {})",
        class_pattern
    );
    Ok(())
}

pub fn resume_vm(handler: &mut JdwpHandler) -> Result<(), String> {
    handler.client.resume_vm()
}

pub fn wait_for_event(handler: &mut JdwpHandler) -> Result<usize, String> {
    handler.client.wait_for_event()
}

pub fn set_log_file(handler: &mut JdwpHandler, path: &str) -> std::io::Result<()> {
    handler.client.set_log_file(path)
}

pub fn flush_log(handler: &mut JdwpHandler) {
    handler.client.flush_log();
}

pub fn print_summary(handler: &JdwpHandler) {
    handler.client.print_trace_summary();
}
