#[path = "protocol_vars.rs"]
pub mod pvars;

use std::convert::TryInto;
use std::str;

use crate::jdwp_handler::jdwp_client::connection::Connection as Conn;
use colored::*;

pub struct IdSizes {
    pub field_id_size: u32,
    pub method_id_size: u32,
    pub object_id_size: u32,
    pub reference_type_id_size: u32,
    pub frame_id_size: u32,
}
impl IdSizes {
    pub fn new() -> Self {
        return IdSizes {
            field_id_size: 0,
            method_id_size: 0,
            object_id_size: 0,
            reference_type_id_size: 0,
            frame_id_size: 0,
        };
    }

    pub fn print(&self) {
        println!("IdSizes:");
        println!("  Field:    {}", self.field_id_size);
        println!("  Method:   {}", self.method_id_size);
        println!("  Object:   {}", self.object_id_size);
        println!("  Ref Type: {}", self.reference_type_id_size);
        println!("  Frame:    {}", self.frame_id_size);
    }
}

pub struct Version {
    pub description: String,
    pub major: u32,
    pub minor: u32,
    pub vm_version: String,
    pub vm_name: String,
}
impl Version {
    pub fn new() -> Self {
        return Version {
            description: "".to_string(),
            major: 0,
            minor: 0,
            vm_version: "".to_string(),
            vm_name: "".to_string(),
        };
    }
    pub fn print(&self) {
        println!("Version:");
        println!("  Description: {}", self.description);
        println!("  Major/Minor: {}/{}", self.major, self.minor);
        println!("  Vm Version: {}", self.vm_version);
        println!("  Vm Name: {}", self.vm_name);
    }
}

use std::collections::HashMap;

pub struct ThreadInfo {
    pub name: String,
    pub color: u8,
}

pub struct Threads {
    pub map: HashMap<u64, ThreadInfo>,
    pub current_color: u8,
}

impl Threads {
    pub fn new() -> Self {
        Threads {
            map: HashMap::new(),
            current_color: 0,
        }
    }

    pub fn get(&self, thread_id: u64) -> Option<&ThreadInfo> {
        self.map.get(&thread_id)
    }

    pub fn insert(&mut self, thread_id: u64, name: String) -> &ThreadInfo {
        let color = self.current_color;
        self.current_color = (self.current_color + 1) % 5;
        self.map.insert(thread_id, ThreadInfo { name, color });
        self.map.get(&thread_id).unwrap()
    }
}

pub fn thread_str(thread_id: u64, thread_name: String, color: u8) -> String {
    let mut tstr = format!("[{}: {}]", thread_name, thread_id);

    tstr = match color {
        0 => tstr.cyan().to_string(),
        1 => tstr.blue().to_string(),
        2 => tstr.magenta().to_string(),
        3 => tstr.yellow().to_string(),
        4 => tstr.white().to_string(),
        _ => tstr.truecolor(169, 30, 204).to_string(),
    };

    return tstr;
}

pub fn slice_to_u32(slice: &[u8]) -> u32 {
    return u32::from_be_bytes(slice.try_into().unwrap());
}

pub fn slice_to_u64(slice: &[u8]) -> u64 {
    return u64::from_be_bytes(slice.try_into().unwrap());
}

pub fn parse_string(buffer: &[u8]) -> String {
    if buffer.len() < 4 {
        return "".to_string();
    }
    let str_length: usize = slice_to_u32(&buffer[0..4]) as usize;
    let end = 4 + str_length;
    if end > buffer.len() {
        return "".to_string();
    }
    let str_raw = &buffer[4..end];
    match str::from_utf8(str_raw) {
        Ok(s) => s.to_string(),
        Err(_) => "".to_string(),
    }
}

pub fn append_u32(buffer: &mut Vec<u8>, value: u32) {
    let mods_vec = value.to_be_bytes();
    for i in 0..4 {
        buffer.push(mods_vec[i]);
    }
}

pub fn append_u64(buffer: &mut Vec<u8>, value: u64) {
    let mods_vec = value.to_be_bytes();
    for i in 0..8 {
        buffer.push(mods_vec[i]);
    }
}

pub fn append_string(buffer: &mut Vec<u8>, class_pattern: &str) {
    append_u32(buffer, class_pattern.len() as u32);

    for b in class_pattern.as_bytes() {
        buffer.push(*b);
    }
}

pub fn parse_idsizes(buffer: &[u8]) -> Result<IdSizes, String> {
    /*
      Packet reply format:
      ("I", "fieldIDSize")
      ("I", "methodIDSize")
      ("I", "objectIDSize")
      ("I", "referenceTypeIDSize")
      ("I", "frameIDSize")
    */

    if buffer.len() < 20 {
        return Err("IDSizes reply was too short".to_string());
    }

    let mut idsizes = IdSizes::new();
    idsizes.field_id_size = slice_to_u32(&buffer[0..4]);
    idsizes.method_id_size = slice_to_u32(&buffer[4..8]);
    idsizes.object_id_size = slice_to_u32(&buffer[8..12]);
    idsizes.reference_type_id_size = slice_to_u32(&buffer[12..16]);
    idsizes.frame_id_size = slice_to_u32(&buffer[16..20]);
    Ok(idsizes)
}

fn parse_string_field(buffer: &[u8]) -> Result<(String, usize), String> {
    if buffer.len() < 4 {
        return Err("String field was too short for length".to_string());
    }

    let str_length = slice_to_u32(&buffer[0..4]) as usize;
    let end = 4 + str_length;
    if end > buffer.len() {
        return Err("String field length exceeds packet size".to_string());
    }

    let value = str::from_utf8(&buffer[4..end])
        .map_err(|e| format!("String field contained invalid UTF-8: {}", e))?
        .to_string();

    Ok((value, end))
}

pub fn parse_version(buffer: &[u8]) -> Result<Version, String> {
    /*
      Packet reply format:
      ('S', "description")
      ('I', "jdwpMajor")
      ('I', "jdwpMinor")
      ('S', "vmVersion")
      ('S', "vmName")
    */

    let mut version = Version::new();
    let (desc, mut it) = parse_string_field(buffer)?;
    version.description = desc.clone();

    if buffer.len() < it + 8 {
        return Err("Version reply was too short for major/minor fields".to_string());
    }
    version.major = slice_to_u32(&buffer[(it)..(it + 4)]);
    version.minor = slice_to_u32(&buffer[(it + 4)..(it + 8)]);
    it += 8;

    let (vm_version, consumed) = parse_string_field(&buffer[it..])?;
    version.vm_version = vm_version.clone();
    it += consumed;

    let (vm_name, _) = parse_string_field(&buffer[it..])?;
    version.vm_name = vm_name.clone();

    Ok(version)
}

static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn get_current_time() -> String {
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let elapsed = start.elapsed();
    let total_ms = elapsed.as_millis() as u64;
    let ms = total_ms % 1000;
    let secs = (total_ms / 1000) % 60;
    let mins = (total_ms / 60000) % 60;
    let hours = total_ms / 3600000;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, ms)
}

pub fn get_thread_by_id(
    con: &mut Conn,
    obj_id_size: u32,
    thread_id: u64,
) -> Result<String, String> {
    let mut data: Vec<u8> = Vec::new();
    if obj_id_size == 4 {
        append_u32(&mut data, thread_id as u32);
    } else {
        append_u64(&mut data, thread_id);
    }

    let packet_id = con.send_packet(pvars::THREADNAME_SIG, &data)?;
    con.read_reply_string(packet_id)
}
