use super::pvars;
use super::utils;
use std::collections::{HashMap, HashSet};

use crate::jdwp_handler::jdwp_client::connection::Connection as Conn;

#[derive(Clone)]
pub struct Method {
    pub ref_type_id: u64, // either u32 or u64
    pub method_id: u64,
    pub name: String,
    pub signature: String,
    pub modbits: u32,
    pub ret_void: bool, // return nothing
    pub native: bool,
}
impl Method {
    pub fn new() -> Self {
        Method {
            ref_type_id: 0, // class_id
            method_id: 0,
            name: "unknown".to_string(),
            signature: "()V".to_string(),
            modbits: 0,
            ret_void: true,
            native: false,
        }
    }

    pub fn print(&self) {
        println!("Method:");
        println!("  Ref Type Id: {}", self.ref_type_id);
        println!("  Method Id:   {}", self.method_id);
        println!("  Name:        {}", self.name);
        println!("  Signature:   {}", self.signature);
        println!("  Modbits:     {}", self.modbits);
        println!("  Native:      {}", self.native);
    }
}

pub struct Methods {
    pub vec: Vec<Method>,
    by_id: HashMap<(u64, u64), usize>,
    fetched_classes: HashSet<u64>,
}
impl Methods {
    pub fn new() -> Self {
        Methods {
            vec: Vec::new(),
            by_id: HashMap::new(),
            fetched_classes: HashSet::new(),
        }
    }

    pub fn push(&mut self, method: Method) {
        let key = (method.ref_type_id, method.method_id);
        if self.by_id.contains_key(&key) {
            return;
        }

        self.vec.push(method);
        self.by_id.insert(key, self.vec.len() - 1);
    }

    pub fn mark_class_fetched(&mut self, ref_type_id: u64) {
        self.fetched_classes.insert(ref_type_id);
    }

    pub fn has_fetched_class(&self, ref_type_id: u64) -> bool {
        self.fetched_classes.contains(&ref_type_id)
    }

    pub fn get_cached_by_id(&self, ref_type_id: u64, method_id: u64) -> Option<Method> {
        self.by_id
            .get(&(ref_type_id, method_id))
            .map(|idx| self.vec[*idx].clone())
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        println!("Methods:");
        for m in &self.vec {
            m.print();
        }
    }
}

fn parse_method(buffer: &[u8], rti: u64, m_id_size: u32) -> Option<(Method, usize)> {
    let mut it: usize = 0;

    let method_id = if m_id_size == 4 {
        if buffer.len() < it + 4 {
            return None;
        }
        utils::slice_to_u32(&buffer[it..it + 4]) as u64
    } else {
        if buffer.len() < it + 8 {
            return None;
        }
        utils::slice_to_u64(&buffer[it..it + 8])
    };
    it += m_id_size as usize;

    if buffer.len() < it + 4 {
        return None;
    }
    let name = utils::parse_string(&buffer[it..]);
    it += name.len() + 4;

    if buffer.len() < it + 4 {
        return None;
    }
    let signature = utils::parse_string(&buffer[it..]);
    it += signature.len() + 4;

    if buffer.len() < it + 4 {
        return None;
    }
    let modbits = utils::slice_to_u32(&buffer[it..it + 4]);
    it += 4;

    let ret_void = signature.as_bytes().last().is_none_or(|&b| b == 86);

    let method = Method {
        ref_type_id: rti,
        method_id,
        name,
        signature,
        modbits,
        ret_void,
        native: modbits & 0x0100 > 0,
    };

    Some((method, it))
}

pub fn fetch_methods_for_class(
    con: &mut Conn,
    methods: &mut Methods,
    m_id_size: u32,
    o_id_size: u32,
    ref_type_id: u64,
) -> Result<usize, String> {
    if methods.has_fetched_class(ref_type_id) {
        return Ok(0);
    }

    let mut data: Vec<u8> = Vec::new();
    if o_id_size == 4 {
        utils::append_u32(&mut data, ref_type_id as u32);
    } else {
        utils::append_u64(&mut data, ref_type_id);
    }
    let packet_id = con.send_packet(pvars::METHODS_SIG, &data)?;

    let buffer = con.read_reply_buffer(packet_id)?;

    if buffer.len() < 4 {
        return Err("Methods reply was too short".to_string());
    }

    let cnt = utils::slice_to_u32(&buffer[0..4]);

    let before = methods.vec.len();
    let mut it: usize = 4;
    for _ in 0..cnt {
        if it >= buffer.len() {
            break;
        }
        match parse_method(&buffer[it..], ref_type_id, m_id_size) {
            Some((method, nit)) => {
                it += nit;
                methods.push(method);
            }
            None => break,
        }
    }

    methods.mark_class_fetched(ref_type_id);
    Ok(methods.vec.len().saturating_sub(before))
}

pub fn get_method_by_id(
    con: &mut Conn,
    methods: &mut Methods,
    m_id_size: u32,
    o_id_size: u32,
    ref_type_id: u64,
    method_id: u64,
) -> Result<Method, String> {
    if let Some(method) = methods.get_cached_by_id(ref_type_id, method_id) {
        return Ok(method);
    }

    fetch_methods_for_class(con, methods, m_id_size, o_id_size, ref_type_id)?;

    if let Some(method) = methods.get_cached_by_id(ref_type_id, method_id) {
        return Ok(method);
    }

    Ok(Method::new())
}
