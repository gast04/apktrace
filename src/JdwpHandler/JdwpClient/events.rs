use super::pvars;
use super::utils;

use crate::jdwp_handler::jdwp_client::connection::Connection as Conn;
use core::convert::TryInto;

pub struct Response {
    pub suspend_policy: u8,
    pub events: Vec<Event>,
}

impl Response {
    pub fn print(&self) {
        println!("Response:");
        println!("  Suspend Policy: {}", self.suspend_policy);
        for event in &self.events {
            event.print();
        }
    }
}

pub struct Event {
    pub kind: u8,
    pub type_tag: u8,
    pub request_id: u32,
    pub thread_id: u64, // can be u32 or u64
    pub location: u64,
    pub class_id: u64,
    pub method_id: u64,
    pub retval: u64,
}

impl Event {
    pub fn print(&self) {
        println!("Event:");
        println!("  kind:       {}", self.kind);
        println!("  type tag:   {}", self.type_tag);
        println!("  request id: {}", self.request_id);
        println!("  thred id:   {}", self.thread_id);
        println!("  location:   {}", self.location);
        println!("  class id:   {}", self.class_id);
        println!("  method_id:  {}", self.method_id);
        println!("  retval:     {}", self.retval); // only print if WRV event
    }
}

fn parse_sized_id(buffer: &[u8], size: u32) -> Option<u64> {
    match size {
        4 if buffer.len() >= 4 => Some(utils::slice_to_u32(&buffer[0..4]) as u64),
        8 if buffer.len() >= 8 => Some(utils::slice_to_u64(&buffer[0..8])),
        _ => None,
    }
}

fn parse_event(
    buffer: &[u8],
    obj_uid_size: u32,
    ref_type_id_size: u32,
    method_id_size: u32,
) -> Option<(Event, usize)> {
    if buffer.len() < 6 {
        return None;
    }

    let event_kind = buffer[0];
    let request_id = utils::slice_to_u32(&buffer[1..5]);
    let mut it: usize = 5;

    let thread_id = parse_sized_id(&buffer[it..], obj_uid_size)?;
    it += obj_uid_size as usize;

    if event_kind != pvars::EVENT_METHOD_ENTRY && event_kind != pvars::EVENT_METHOD_EXIT_WRV {
        return None;
    }

    if buffer.len() < it + 1 {
        return None;
    }
    let type_tag = buffer[it];
    it += 1;

    let class_id = parse_sized_id(&buffer[it..], ref_type_id_size)?;
    it += ref_type_id_size as usize;

    let method_id = parse_sized_id(&buffer[it..], method_id_size)?;
    it += method_id_size as usize;

    if buffer.len() < it + 8 {
        return None;
    }
    let loc_index = utils::slice_to_u64(&buffer[it..it + 8]);
    it += 8;

    let mut retval: u64 = 0;
    if event_kind == pvars::EVENT_METHOD_EXIT_WRV {
        if buffer.len() <= it {
            return None;
        }
        match buffer[it] {
            66 | 90 if buffer.len() >= it + 2 => {
                retval = buffer[it + 1] as u64;
                it += 2;
            }
            67 | 83 if buffer.len() >= it + 3 => {
                retval = u16::from_be_bytes(buffer[it + 1..it + 3].try_into().unwrap()) as u64;
                it += 3;
            }
            73 | 70 if buffer.len() >= it + 5 => {
                retval = utils::slice_to_u32(&buffer[it + 1..it + 5]) as u64;
                it += 5;
            }
            74 | 68 if buffer.len() >= it + 9 => {
                retval = utils::slice_to_u64(&buffer[it + 1..it + 9]);
                it += 9;
            }
            76 | 91 | 115 | 116 | 103 | 108 | 99 => {
                let value_size = obj_uid_size as usize;
                if buffer.len() < it + 1 + value_size {
                    return None;
                }
                retval = parse_sized_id(&buffer[it + 1..], obj_uid_size).unwrap_or(0);
                it += 1 + value_size;
            }
            86 => {
                it += 1;
            }
            _ => return None,
        };
    }

    let event = Event {
        kind: event_kind,
        request_id: request_id,
        thread_id: thread_id,
        type_tag: type_tag,
        class_id: class_id,
        method_id: method_id,
        location: loc_index,
        retval: retval,
    };

    Some((event, it))
}

pub fn parse_event_response(
    buffer: &[u8],
    obj_uid_size: u32,
    ref_type_id_size: u32,
    method_id_size: u32,
) -> Response {
    let mut response = Response {
        suspend_policy: if buffer.len() > 0 { buffer[0] } else { 0 },
        events: Vec::new(),
    };

    if buffer.len() < 6 {
        return response;
    }

    let event_cnt = utils::slice_to_u32(&buffer[1..5]);
    let mut it: usize = 5;
    for _ in 0..event_cnt {
        match parse_event(
            &buffer[it..],
            obj_uid_size,
            ref_type_id_size,
            method_id_size,
        ) {
            Some((event, consumed)) => {
                response.events.push(event);
                it += consumed;
            }
            None => break,
        }
    }

    return response;
}

pub fn class_match_event(
    con: &mut Conn,
    class_pattern: &str,
    event_kind: u8,
) -> Result<u32, String> {
    /*
      Packet data Format:
        B Event Kind
        B Suspend policy
        I modifiers
        [
          -> modifer depends on type

          B modifier kind (5) ClassMatch
          S classPattern
        ]
      all modifiers have to match, so classmatch allows
      only one
    */

    let mut packet_data: Vec<u8> = Vec::new();
    packet_data.push(event_kind);
    packet_data.push(pvars::SUSPEND_EVENT_THREAD);
    utils::append_u32(&mut packet_data, 1);
    packet_data.push(pvars::MODKIND_CLASS_MATCH);
    utils::append_string(&mut packet_data, class_pattern);

    let packet_id = con.send_packet(pvars::EVENTSET_SIG, &packet_data)?;
    con.read_reqid(packet_id)
}

pub fn class_exclude_event(con: &mut Conn, event_kind: u8) -> Result<u32, String> {
    /*
      Packet data Format:
        B Event Kind
        B Suspend policy
        I modifiers
        [
          -> modifer depends on type

          B modifier kind (6) ClassExclude
          S classPattern
        ]
    */

    let mut packet_data: Vec<u8> = Vec::new();
    packet_data.push(event_kind);
    packet_data.push(pvars::SUSPEND_EVENT_THREAD);
    utils::append_u32(&mut packet_data, 9);

    // unwanted java classes
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "java.*");
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "javax.*");
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "sun.*");
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "com.sun.*");

    // android specific classes
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "dalvik.system.*");
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "libcore.*");
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "android.*");
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "com.android.*");
    packet_data.push(pvars::MODKIND_CLASS_EXCLUDE);
    utils::append_string(&mut packet_data, "androidx.*");

    let packet_id = con.send_packet(pvars::EVENTSET_SIG, &packet_data)?;
    con.read_reqid(packet_id)
}
