import struct

# TODO: move to utils

def createstring(self, data):
  buf = self.buildstring(data)
  self.socket.sendall( create_packet(self.id, CREATESTRING_SIG, data=buf) )
  buf = read_reply(self.socket)
  return self.parse_entries(buf, [(self.objectIDSize, "objId")], False)

def buildstring(self, data):
  return struct.pack(">I", len(data)) + data

def readstring(self, data):
  size = struct.unpack(">I", data[:4])[0]
  return data[4:4+size]

def solve_string(self, objId):
  self.socket.sendall( create_packet(self.id, STRINGVALUE_SIG, data=objId) )
  buf = read_reply(self.socket)
  if len(buf):
    return self.readstring(buf)
  else:
    return ""