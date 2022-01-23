import struct
import time
from typing import Tuple

from .utils import dbgPrint, dbgError

# JDWP ERROR codes
INVALID_SLOT              = 35
INVALID_EVENT_TYPE        = 102
NATIVE_METHOD             = 511

def rawSend(self, data):
  self.socket.send(data)

def rawRead(self, length):
  return self.socket.recv(length)

def sendPacket(self, cmdsig, data=""):
  pkt = createPacket(self, cmdsig, data)
  self.socket.sendall(pkt)

def createPacket(self, cmdsig: Tuple[int, int], data: bytes):
  '''
    (https://docs.oracle.com/en/java/javase/15/docs/specs/jdwp/jdwp-spec.html)
    Header Structure
      packet length (int)
      packet id     (int)
      flags         (byte)      # only 0x80 is defined (reply packet)
      command set   (byte)
      command       (byte)
      data          (user defined)

    Header length: 11 bytes
  '''
  pktlen = len(data) + 11
  cmdset, cmd = cmdsig

  pkt = struct.pack(">IIccc",
      pktlen, self.id,
      b'\x00',            # send flags
      bytes([cmdset]),
      bytes([cmd])
    )

  # remove in future
  if type(data) == str:
    data = data.encode("UTF-8")

  pkt += data
  self.id += 2            # must be unique among all sent packets
  return pkt


def _readReplyHeader(socket) -> str:
  while True:
    header = socket.recv(11)
    if len(header) == 11:
      return header

    # only happens in error case
    dbgPrint("Waiting for reply...")
    time.sleep(0.1)

def waitReply(self):
  _readReplyHeader(self.socket)

def readReplyReqId(self):
  reply = self.readReplyBuf()
  return struct.unpack(">I", reply)[0]

def readReplyString(self):
  reply = self.readReplyBuf()
  size = struct.unpack(">I", reply[:4])[0]
  return reply[4:4+size]

def readReplyBuf(self):
  header = _readReplyHeader(self.socket)
  pktlen, id, flags, errcode = struct.unpack(">IIcH", header)

  if flags == '\x80': # REPLY_PACKET_TYPE
    if errcode:
      if errcode == INVALID_EVENT_TYPE:
        dbgError("102 - INVALID_EVENT_TYPE")
      elif errcode == INVALID_SLOT:
        dbgError("35 - INVALID_SLOT")
      elif errcode == NATIVE_METHOD:
        dbgError("511 - NATIVE METHOD")
      else:
        dbgError("Unhandled errorcode {}".format(errcode))
      return b""

  buf = b""
  while len(buf) + 11 < pktlen:
    data = self.socket.recv(1024)
    if len(data):
      buf += data
    else:
      time.sleep(1)

  return buf
