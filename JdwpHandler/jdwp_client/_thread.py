import struct
from ._protvars import *

def allthreads(self):
  try:
    getattr(self, "threads")
  except :
    self.socket.sendall( create_packet(self.id, ALLTHREADS_SIG) )
    buf = read_reply(self.socket)
    formats = [ (self.objectIDSize, "threadId")]
    self.threads = self.parse_entries(buf, formats)
  finally:
    return self.threads

def get_thread_by_name(self, name):
  self.allthreads()
  for t in self.threads:
    threadId = self.format(self.objectIDSize, t["threadId"])
    self.socket.sendall( create_packet(self.id, THREADNAME_SIG, data=threadId) )
    buf = read_reply(self.socket)
    if len(buf) and name == self.readstring(buf):
      return t
  return None

def get_thread_by_id(self, thread_id):

  if thread_id in self.threads:
    return self.threads[thread_id]

  threadId = self.format(self.objectIDSize, thread_id)
  self.con.sendPacket(THREADNAME_SIG, threadId)
  thread_name = self.con.readReplyString()

  # append to threads list
  self.threads[thread_id] = thread_name
  return thread_name

def query_thread(self, threadId, kind):
  data = self.format(self.objectIDSize, threadId)
  self.socket.sendall( create_packet(self.id, kind, data=data) )
  buf = read_reply(self.socket)
  return

def suspend_thread(self, threadId):
  return self.query_thread(threadId, THREADSUSPEND_SIG)

def status_thread(self, threadId):
  return self.query_thread(threadId, THREADSTATUS_SIG)

def resume_thread(self, threadId):
  return self.query_thread(threadId, THREADRESUME_SIG)

def send_thread_cmd(self, cmd_code, data):
  self.socket.sendall( create_packet(self.id, cmd_code, data=data) )
  buf = read_reply(self.socket)

  frames_cnt = struct.unpack(">I", buf[0:4])[0]

  i = 4
  frames = []
  for _ in range(frames_cnt):
    # frameIDsize == 8

    frameID = struct.unpack(">Q", buf[i:i+8])[0]
    i += 8

    loc_type_tag = buf[i]
    class_id = struct.unpack(">Q", buf[i+1:i+9])[0]
    method_id = struct.unpack(">I", buf[i+9:i+13])[0]
    index = struct.unpack(">Q", buf[i+13:i+21])[0]
    i += 21

    frames.append((frameID, loc_type_tag, class_id, method_id, index))

  return frames
