use core::convert::TryInto;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::str;
use std::time::Duration;

const HEADER_LEN: usize = 11;
const READ_TIMEOUT_SECS: u64 = 30;
const CONNECT_TIMEOUT_SECS: u64 = 10;
const REPLY_FLAG: u8 = 0x80;
const MAX_PENDING_COMMAND_PACKETS: usize = 1024;

struct PktHeader {
    len: u32, // lenght without header
    id: u32,
    flags: u8,
    errcode: u16,
}

impl PktHeader {
    pub fn print(&self) {
        println!("Header:");
        println!("  len:  {}", self.len);
        println!("  id:   {}", self.id);
        println!("  flg:  {}", self.flags);
        println!("  errc: {}", self.errcode);
    }
}

// Struct for connection
pub struct Connection {
    con: TcpStream,
    packet_id: u32,
    pending_command_packets: VecDeque<Vec<u8>>,
    verbose: bool,
}

// add functions to the struct
impl Connection {
    pub fn new(host: &str, port: u16, verbose: bool) -> Result<Self, String> {
        let hostport: String = format!("{}:{}", host, port);
        let addr = hostport
            .to_socket_addrs()
            .map_err(|e| format!("Could not resolve {}: {}", hostport, e))?
            .next()
            .ok_or_else(|| format!("Could not resolve {}", hostport))?;

        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .map_err(|e| format!("Could not connect to {}: {}", hostport, e))?;

        let timeout = Some(Duration::from_secs(READ_TIMEOUT_SECS));
        stream
            .set_read_timeout(timeout)
            .map_err(|e| format!("Could not set read timeout: {}", e))?;
        stream
            .set_write_timeout(timeout)
            .map_err(|e| format!("Could not set write timeout: {}", e))?;

        Ok(Connection {
            con: stream,
            packet_id: 1,
            pending_command_packets: VecDeque::new(),
            verbose,
        })
    }

    fn dbg_print(&self, msg: &str) {
        if self.verbose {
            println!("[Connection] {}", msg);
        }
    }

    pub fn send_raw(&mut self, data: &[u8]) -> Result<(), String> {
        self.con
            .write_all(data)
            .map_err(|e| format!("Could not send raw data: {}", e))
    }

    pub fn send_packet(&mut self, cmd_sig: (u8, u8), data: &[u8]) -> Result<u32, String> {
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

        let packet_len: u32 = (data.len() as u32) + HEADER_LEN as u32;
        let plen_vec = packet_len.to_be_bytes();

        // convert packet id to be_bytes
        let packet_id = self.packet_id;
        let pid_vec = packet_id.to_be_bytes();

        // specify capacity does not set length of vector
        let mut packet: Vec<u8> = Vec::with_capacity(packet_len as usize);

        // fill packet vector
        for i in 0..4 {
            packet.push(plen_vec[i]);
        }
        for i in 0..4 {
            packet.push(pid_vec[i]);
        }
        packet.push(0); // flags
        packet.push(cmd_sig.0); // cmd set
        packet.push(cmd_sig.1); // cmd
        for i in 0..data.len() {
            packet.push(data[i]);
        }

        // TODO: pretty print final packet
        self.dbg_print(&format!("Final Packet: {:?}", packet));

        self.con
            .write_all(&packet[..])
            .map_err(|e| format!("Could not send packet id {}: {}", packet_id, e))?;

        // inc packet id, must be unique among all sent packets
        self.packet_id = self.packet_id.wrapping_add(1);

        Ok(packet_id)
    }

    pub fn read_raw(&mut self, data: &mut [u8]) -> Result<(), String> {
        if data.is_empty() {
            return Ok(());
        }

        let mut read = 0;
        loop {
            match self.con.read(&mut data[read..]) {
                Ok(0) => {
                    self.dbg_print("Connection closed");
                    return Err("Connection closed".to_string());
                }
                Ok(r) => {
                    read += r;
                    if read == data.len() {
                        break;
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut
                    {
                        self.dbg_print("Read timeout!");
                        return Err("Read timeout".to_string());
                    } else {
                        self.dbg_print(&format!("Read error: {}", e));
                        return Err(format!("Read error: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    fn read_reply_header(&mut self) -> Result<PktHeader, String> {
        let mut header: [u8; HEADER_LEN] = [0; HEADER_LEN];
        self.read_raw(&mut header)?;
        self.dbg_print(&format!("Header Raw: {:?}", header));

        // parese header
        let packet_len = u32::from_be_bytes(header[0..4].try_into().unwrap());
        if packet_len < HEADER_LEN as u32 {
            return Err(format!("Invalid JDWP packet length: {}", packet_len));
        }

        let pkt_header = PktHeader {
            len: packet_len - HEADER_LEN as u32, // store without header length
            id: u32::from_be_bytes(header[4..8].try_into().unwrap()),
            flags: header[8],
            errcode: u16::from_be_bytes(header[9..11].try_into().unwrap()),
        };

        if self.verbose {
            pkt_header.print();
        }

        Ok(pkt_header)
    }

    fn validate_reply(&self, pkt_header: &PktHeader, expected_id: u32) -> Result<(), String> {
        if pkt_header.flags & REPLY_FLAG == 0 {
            return Err(format!(
                "Expected reply for packet {}, got command packet {}",
                expected_id, pkt_header.id
            ));
        }

        if pkt_header.id != expected_id {
            return Err(format!(
                "Expected reply id {}, got {}",
                expected_id, pkt_header.id
            ));
        }

        if pkt_header.errcode != 0 {
            return Err(format!(
                "JDWP command {} failed with error code {}",
                expected_id, pkt_header.errcode
            ));
        }

        Ok(())
    }

    fn is_reply(pkt_header: &PktHeader) -> bool {
        pkt_header.flags & REPLY_FLAG != 0
    }

    fn read_packet(&mut self) -> Result<(PktHeader, Vec<u8>), String> {
        let pkt_header = self.read_reply_header()?;
        // create vector and fill with items to get length
        let mut data_buffer: Vec<u8> = vec![0; pkt_header.len as usize];
        self.read_raw(&mut data_buffer[..])?;

        Ok((pkt_header, data_buffer))
    }

    fn queue_command_packet(&mut self, packet_id: u32, data_buffer: Vec<u8>) -> Result<(), String> {
        if self.pending_command_packets.len() >= MAX_PENDING_COMMAND_PACKETS {
            return Err(format!(
                "Too many pending command packets while waiting for reply {}",
                packet_id
            ));
        }

        self.dbg_print(&format!(
            "Queued command packet {} while waiting for reply",
            packet_id
        ));
        self.pending_command_packets.push_back(data_buffer);
        Ok(())
    }

    fn read_expected_reply(&mut self, expected_id: u32) -> Result<Vec<u8>, String> {
        loop {
            let (pkt_header, data_buffer) = self.read_packet()?;
            if Self::is_reply(&pkt_header) {
                self.validate_reply(&pkt_header, expected_id)?;
                return Ok(data_buffer);
            }

            self.queue_command_packet(pkt_header.id, data_buffer)?;
        }
    }

    pub fn wait_reply(&mut self, expected_id: u32) -> Result<(), String> {
        self.read_expected_reply(expected_id).map(|_| ())
    }

    pub fn read_buffer(&mut self) -> Result<Vec<u8>, String> {
        if let Some(data_buffer) = self.pending_command_packets.pop_front() {
            return Ok(data_buffer);
        }

        loop {
            let (pkt_header, data_buffer) = self.read_packet()?;
            if !Self::is_reply(&pkt_header) {
                return Ok(data_buffer);
            }

            return Err(format!(
                "Unexpected reply packet {} while waiting for an event",
                pkt_header.id
            ));
        }
    }

    pub fn read_reply_buffer(&mut self, expected_id: u32) -> Result<Vec<u8>, String> {
        self.read_expected_reply(expected_id)
    }

    pub fn read_reply_string(&mut self, expected_id: u32) -> Result<String, String> {
        let buffer = self.read_reply_buffer(expected_id)?;

        if buffer.len() < 4 {
            return Err(format!(
                "Reply {} was too short for string length",
                expected_id
            ));
        }

        let str_len = u32::from_be_bytes(buffer[0..4].try_into().unwrap()) as usize;
        if buffer.len() < str_len + 4 {
            return Err(format!(
                "Reply {} string length exceeds packet size",
                expected_id
            ));
        }

        let str_raw = &buffer[4..(str_len + 4)];
        str::from_utf8(str_raw)
            .map(|s| s.to_string())
            .map_err(|e| format!("Reply {} contained invalid UTF-8: {}", expected_id, e))
    }

    pub fn read_reqid(&mut self, expected_id: u32) -> Result<u32, String> /* return parsed request id*/
    {
        let buffer = self.read_reply_buffer(expected_id)?;

        if buffer.len() < 4 {
            return Err(format!(
                "Reply {} was too short for request id",
                expected_id
            ));
        }
        Ok(u32::from_be_bytes(buffer[0..4].try_into().unwrap()))
    }
}
