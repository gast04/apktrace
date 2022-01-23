use std::str;
use std::net::TcpStream;
use std::io::{Read, Write};
use core::convert::TryInto;

const HEADER_LEN: u8 = 11;

struct PktHeader {
  len: u32,     // lenght without header
  id: u32,
  flags: u8,
  errcode: u16,
}

impl PktHeader {
  pub fn print(&self) {
    println!("Header:");
    println!("  len:  {}",self.len);
    println!("  id:   {}",self.id);
    println!("  flg:  {}",self.flags);
    println!("  errc: {}",self.errcode);
  }
}

// Struct for connection
pub struct Connection {
  con: TcpStream,
  packet_id: u32,
  verbose: bool,
}

// add functions to the struct
impl Connection {

  // create new tcp connection
  pub fn new(host: &str, port: u16, verbose: bool) -> Result<Self, i8> {

    let hostport: String = format!("{}:{}", host, port);
    let result = TcpStream::connect(&hostport);
    if !result.is_ok() {
      return Err(-1);
    }
    // connected, retrieve tcpstream
    let stream = result.unwrap();
    return Ok(Connection { con: stream, packet_id: 0, verbose: verbose });
  }

  fn dbg_print(&self, msg: &str) {
    if self.verbose {
      println!("[Connection] {}",msg);
    }
  }

  pub fn send_raw(&mut self, data: &[u8]) -> bool {
    let res = self.con.write(data);
    if !res.is_ok() {
      self.dbg_print(&format!("Could not send raw data: {:?}", data));
      return false;
    }
    return true;
  }

  pub fn send_packet(&mut self, cmd_sig: (u8, u8), data: &[u8]) -> bool {
    /*
      (https://docs.oracle.com/en/java/javase/15/docs/specs/jdwp/jdwp-spec.html)
      Header Structure
        packet length (int)
        packet id     (int)
        flags         (byte)      # only 0x80 is defined (reply packet)
        command set   (byte)
        command       (byte)
        data          (user defined)

      Header length: 11 bytes
    */

    let packet_len:u32 = (data.len() as u32) + HEADER_LEN as u32;
    let plen_vec = packet_len.to_be_bytes();

    // convert packet id to be_bytes
    let pid_vec = self.packet_id.to_be_bytes();

    // specify capacity does not set length of vector
    let mut packet: Vec<u8> = Vec::with_capacity(packet_len as usize);

    // fill packet vector
    for i in 0..4 { packet.push(plen_vec[i]); }
    for i in 0..4 { packet.push(pid_vec[i]); }
    packet.push(0);           // flags
    packet.push(cmd_sig.0);   // cmd set
    packet.push(cmd_sig.1);   // cmd
    for i in 0..data.len() { packet.push(data[i]); }

    // TODO: pretty print final packet
    self.dbg_print(&format!("Final Packet: {:?}", packet));

    let res = self.con.write(&packet[..]); // pass as &[u8]
    if !res.is_ok() {
      self.dbg_print("Could not send data!" /* TODO: print data */);
      return false;
    }

    // inc packet id, must be unique among all sent packets
    self.packet_id += 2;

    return true;
  }

  pub fn read_raw(&mut self, data: &mut [u8]) -> bool {
    // read size of array much data
    
    let mut read = 0;
    loop {
      let res = self.con.read(&mut data[read..]);
      if !res.is_ok() {
        self.dbg_print("Could not read raw data!");
        return false;
      }

      // probably can be improved
      let r = res.unwrap();
      read += r;
      if read == data.len() {
        break;
      }
    }

    return true;
  }

  fn read_reply_header(&mut self) -> Result<PktHeader, i8> {
    let mut header:[u8; HEADER_LEN as usize] = [0; HEADER_LEN as usize];
    let res = self.con.read(&mut header);
    if !res.is_ok() {
      self.dbg_print("Could not read header data!");
      return Err(-1);
    }
    self.dbg_print(&format!("Header Raw: {:?}", header));

    // parese header
    let pkt_header = PktHeader {
      len:     u32::from_be_bytes(header[0..4].try_into().unwrap())
                        - HEADER_LEN as u32, // store without header length
      id:      u32::from_be_bytes(header[4..8].try_into().unwrap()),
      flags:   header[8],
      errcode: u16::from_be_bytes(header[9..11].try_into().unwrap()),
    };

    if self.verbose {
      pkt_header.print();
    }

    // check error handling


    return Ok(pkt_header);
  }

  pub fn wait_reply(&mut self) {
    let _ = self.read_reply_header();
  }

  pub fn read_buffer(&mut self) -> Result<Vec<u8>, i8> {
    
    let res = self.read_reply_header();
    if !res.is_ok() {
      return Err(-1);
    }
    let pkt_header = res.unwrap();

    // create vector and fill with items to get length
    let mut data_buffer: Vec<u8> = Vec::with_capacity((pkt_header.len+1) as usize);
    for _ in 0..pkt_header.len {data_buffer.push(0);}
    if !self.read_raw(&mut data_buffer[..]) {
      return Err(-1);
    }

    return Ok(data_buffer);
  }

  pub fn read_string(&mut self) -> String {
    let res = self.read_buffer();
    if !res.is_ok() {
      return "".to_string();
    }
    let buffer = res.unwrap();

    let str_len = u32::from_be_bytes(buffer[0..4].try_into().unwrap()) as usize;
    let str_raw = &buffer[4..(str_len+4)];
    let res = str::from_utf8(str_raw);
    if !res.is_ok() {
      return "".to_string();
    }

    return res.unwrap().to_string();
  }

  pub fn read_reqid(&mut self) -> u32 /* return parsed request id*/ {
        
    let res = self.read_reply_header();
    if !res.is_ok() {
      return 0;
    }

    let mut buffer: [u8;4] = [0; 4];
    if !self.read_raw(&mut buffer) {
      return 0;
    }
    return u32::from_be_bytes(buffer.try_into().unwrap());
  }
}
