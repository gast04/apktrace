import time
import sys
import struct
import fuckpy3
from termcolor import colored

from ._utils import *
from ._protvars import *
# create_packet,  read_reply


################################################################################
# JDWP client class
# TODO: rework needed, do better separation and cleaner extraction
################################################################################

def start(self):
  self.con.connect()  # connect to server
  self.handshake()
  self.suspendvm()
  self.idsizes()
  self.getVersion()
  self.fetchAllClasses()   # get all initial classes

def handshake(self):
  self.con.rawSend(HANDSHAKE)
  if self.con.rawRead(len(HANDSHAKE)) != HANDSHAKE:
    dbgError("JDWP-Handshake failed")
    sys.exit(-1)

def leave(self):
  self.con.close()
  return

@property
def version(self):
  return "%s - %s" % (self.vmName, self.vmVersion)

def getVersion(self):
  self.con.sendPacket(VERSION_SIG)
  buf = self.con.readReplyBuf()

  # append as attribute to JdwpClient class
  for entry in self.parse_entries(buf, version_formats, False):
    for name,value  in entry.items():
      setattr(self, name, value)

  # print out JDWP version
  for line in self.description.split(b"\n"):
    dbgPrint(line.str())
  return



def getBpValues(self):
  return self.bp_class_id, self.bp_method_id, self.bp_thread_id, self.bp_location

def parse_entries(self, buf, formats, explicit=True):
  entries = []
  index = 0

  if explicit:
    nb_entries = struct.unpack(">I", buf[:4])[0]
    buf = buf[4:]
  else:
    nb_entries = 1

  for i in range(nb_entries):
    data = {}
    for fmt, name in formats:
      if fmt == "L" or fmt == 8:
        data[name] = int(struct.unpack(">Q",buf[index:index+8]) [0])
        index += 8
      elif fmt == "I" or fmt == 4:
        data[name] = int(struct.unpack(">I", buf[index:index+4])[0])
        index += 4
      elif fmt == 'S':
        # string type prefixed with 4 byte of length
        l = struct.unpack(">I", buf[index:index+4])[0]
        data[name] = buf[index+4:index+4+l]
        index += 4+l
      elif fmt == 'C':
        data[name] = ord(struct.unpack(">c", chr(buf[index]).bytes())[0])
        index += 1
      elif fmt == 'Z':
        t = ord(struct.unpack(">c", buf[index])[0])
        if t == 115: # 's'
          s = self.solve_string(buf[index+1:index+9])
          data[name] = s
          index+=9
        elif t == 73: # 'I'
          data[name] = struct.unpack(">I", buf[index+1:index+5])[0]
          buf = struct.unpack(">I", buf[index+5:index+9])
          index=0
      else:
        print("Error")
        sys.exit(1)

    entries.append( data )

  return entries

def format(self, fmt, value):
  if fmt == "L" or fmt == 8:
    return struct.pack(">Q", value)
  elif fmt == "I" or fmt == 4:
    return struct.pack(">I", value)

  raise Exception("Unknown format")

def unformat(self, fmt, value):
  if fmt == "L" or fmt == 8:
    return struct.unpack(">Q", value[:8])[0]
  elif fmt == "I" or fmt == 4:
    return struct.unpack(">I", value[:4])[0]
  else:
    raise Exception("Unknown format")
  return

def idsizes(self):
  self.con.sendPacket(IDSIZES_SIG)
  buf = self.con.readReplyBuf()
  formats = [ ("I", "fieldIDSize"),
              ("I", "methodIDSize"),
              ("I", "objectIDSize"),
              ("I", "referenceTypeIDSize"),
              ("I", "frameIDSize")
            ]

  # add id sizes as attribute to JdwpClient class
  for entry in self.parse_entries(buf, formats, False):
    for name,value  in entry.items():
      #if name == "methodIDSize":
      #  value = 4
      setattr(self, name, value)
  return

def getfields(self, refTypeId):
  if not self.fields.has_key( refTypeId ):
    refId = self.format(self.referenceTypeIDSize, refTypeId)
    self.socket.sendall( create_packet(self.id, FIELDS_SIG, data=refId) )
    buf = read_reply(self.socket)
    formats = [ (self.fieldIDSize, "fieldId"),
                ('S', "name"),
                ('S', "signature"),
                ('I', "modbits")]
    self.fields[refTypeId] = self.parse_entries(buf, formats)
  return self.fields[refTypeId]

def getvalue(self, refTypeId, fieldId):
  data = self.format(self.referenceTypeIDSize, refTypeId)
  data+= struct.pack(">I", 1)
  data+= self.format(self.fieldIDSize, fieldId)
  self.socket.sendall( create_packet(self.id, GETVALUES_SIG, data=data) )
  buf = read_reply(self.socket)
  formats = [ ("Z", "value") ]
  field = self.parse_entries(buf, formats)[0]
  return field

def invokestatic(self, classId, threadId, methId, *args):
  data = self.format(self.referenceTypeIDSize, classId)
  data+= self.format(self.objectIDSize, threadId)
  data+= self.format(self.methodIDSize, methId)
  data+= struct.pack(">I", len(args))
  for arg in args:
    data+= arg
  data+= struct.pack(">I", 0)

  self.socket.sendall( create_packet(self.id, INVOKESTATICMETHOD_SIG, data=data) )
  buf = read_reply(self.socket)
  return buf

def invoke(self, objId, threadId, classId, methId, *args):
  data = self.format(self.objectIDSize, objId)
  data+= self.format(self.objectIDSize, threadId)
  data+= self.format(self.referenceTypeIDSize, classId)
  data+= self.format(self.methodIDSize, methId)
  data+= struct.pack(">I", len(args))
  for arg in args:
    data+= arg
  data+= struct.pack(">I", 0)

  self.socket.sendall( create_packet(self.id, INVOKEMETHOD_SIG, data=data) )
  buf = read_reply(self.socket)
  return buf

def send_stack_cmd(self, cmd_code, data, quick_ret):
  self.socket.sendall( create_packet(self.id, cmd_code, data=data) )
  buf = read_reply(self.socket)
  return buf

# Capabilities
def send_caps(self, caps_code, *args):
  self.socket.sendall( create_packet(self.id, caps_code, data=b"") )
  buf = read_reply(self.socket)
  return struct.unpack(">I", buf)[0]
