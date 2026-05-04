#[path = "protocol_vars.rs"]
mod pvars;
#[path = "utils.rs"]
mod utils;

use crate::jdwp_handler::jdwp_client::connection::Connection as Conn;

pub struct Class {
    pub ref_type_tag: u8,
    pub ref_type_id: u64, // either u32 or u64
    pub signature: String,
    pub status: u32,
}
impl Class {
    pub fn new() -> Self {
        return Class {
            ref_type_tag: 0,
            ref_type_id: 0,
            signature: "".to_string(),
            status: 0,
        };
    }

    pub fn print(&self) {
        println!("Class:");
        println!("  Ref Type Tag: {}", self.ref_type_tag);
        println!("  Ref Typ Id:   {}", self.ref_type_id);
        println!("  Signature:    {}", self.signature);
        println!("  Status:       {}", self.status)
    }
}

pub struct Classes {
    pub vec: Vec<Class>,
}
impl Classes {
    pub fn new() -> Self {
        return Classes { vec: Vec::new() };
    }
}

fn parse_class(slice: &[u8], tis: u32) -> Option<(Class, u32)> {
    let min_len = 1 + tis as usize + 4 + 4; // tag + id + str_len + status
    if slice.len() < min_len {
        return None;
    }

    let mut class = Class::new();
    class.ref_type_tag = slice[0];

    if tis == 4 {
        if slice.len() < 5 {
            return None;
        }
        class.ref_type_id = utils::slice_to_u32(&slice[1..5]) as u64;
    } else {
        if slice.len() < 9 {
            return None;
        }
        class.ref_type_id = utils::slice_to_u64(&slice[1..9]);
    }

    let str_start = 1 + tis as usize;
    if slice.len() < str_start + 4 {
        return None;
    }

    let signature = utils::parse_string(&slice[str_start..]);
    let it = str_start + 4 + signature.len();

    if slice.len() < it + 4 {
        return None;
    }

    class.signature = signature;
    class.status = utils::slice_to_u32(&slice[it..it + 4]);

    Some((class, (it + 4) as u32))
}

pub fn fetch_all_classes(
    con: &mut Conn,
    classes: &mut Classes,
    type_id_size: u32,
) -> Result<(), String> {
    let packet_id = con.send_packet(pvars::ALLCLASSES_SIG, b"")?;
    let buffer = con.read_reply_buffer(packet_id)?;

    if buffer.len() < 4 {
        return Err("AllClasses reply was too short".to_string());
    }

    let cnt = utils::slice_to_u32(&buffer[0..4]);

    let mut next_entry: u64 = 0;
    for _ in 0..cnt {
        let offset = 4 + next_entry as usize;
        if offset >= buffer.len() {
            break;
        }
        match parse_class(&buffer[offset..], type_id_size) {
            Some((class, size)) => {
                if !class.signature.is_empty() {
                    classes.vec.push(class);
                }
                next_entry += size as u64;
            }
            None => break,
        }
    }
    Ok(())
}

pub fn get_name_by_id(
    con: &mut Conn,
    classes: &mut Classes,
    ref_tis: u32,
    ref_type_id: u64,
) -> Result<String, String> {
    for class in &classes.vec {
        if class.ref_type_id == ref_type_id {
            return Ok(class.signature.clone());
        }
    }

    // clear all classes and reload again
    classes.vec.clear();
    fetch_all_classes(con, classes, ref_tis)?;
    for class in &classes.vec {
        if class.ref_type_id == ref_type_id {
            return Ok(class.signature.clone());
        }
    }

    Ok("unknownClass;".to_string())
}
