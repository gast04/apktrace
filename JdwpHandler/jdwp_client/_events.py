import struct
from ._protvars import *
from ._utils import *

'''
  TODO: documentation about how events look
'''

def send_event(self, eventCode, *args):
  data = b""
  data+= chr( eventCode ).bytes()
  data+= chr( SUSPEND_ALL ).bytes()
  data+= struct.pack(">I", len(args))

  #for kind, option in args:
  #    data+= chr( kind ).bytes()  # modKind byte
  #    data+= option.bytes()       # count   int

  for a in args:
    data += chr(a[0]).bytes() # modKind byte

    for b in a[1:]:
      data += b.bytes()

  self.socket.sendall( create_packet(self.id, EVENTSET_SIG, data=data) )
  buf = read_reply(self.socket)
  return struct.unpack(">I", buf)[0]

def clear_event(self, eventCode, rId):
  data = chr(eventCode)
  data+= struct.pack(">I", rId)
  self.socket.sendall( create_packet(self.id, EVENTCLEAR_SIG, data=data) )
  read_reply(self.socket)
  return

def clear_events(self):
  self.socket.sendall( create_packet(self.id, EVENTCLEARALL_SIG) )
  read_reply(self.socket)
  return

def wait_for_event(self):
  return self.con.readReplyBuf()

def parse_event_response(self, buf):
  ret_val = -1

  if buf == None or len(buf) == 0:
    dbgPrint("parse_event_response empty buffer")
    return -1, -1, -1

  suspend_policy = buf[0]
  events_cnt     = struct.unpack(">I",buf[1:5])[0]
  if events_cnt != 1:
    # 
    dbgPrint("MORE EVENTS: " + str(events_cnt))
    dbgPrint(buf)
    sys.exit(-1)

  event_kind     = buf[5]
  req_ID         = struct.unpack(">I",buf[6:10])[0]

  if self.objectIDSize == 8:
    thread_ID    = struct.unpack(">Q",buf[10:18])[0]
    i = 18
  else:
    thread_ID    = struct.unpack(">I",buf[10:14])[0]
    i = 14

  if (event_kind == EVENT_SINGLE_STEP or
      event_kind == EVENT_BREAKPOINT  or
      event_kind == EVENT_METHOD_ENTRY or 
      event_kind == EVENT_METHOD_EXIT_WRV):

    typeTag         = buf[i]
    class_ID        = struct.unpack(">Q",buf[i+1:i+9])[0]
    
    if self.methodIDSize == 8:
      method_ID     = struct.unpack(">Q",buf[i+9:i+17])[0]
      i += 17
    else:
      method_ID     = struct.unpack(">I",buf[i+9:i+13])[0]
      i += 13
    loc_index       = struct.unpack(">Q",buf[i:i+8])[0]
    i += 8

    self.bp_class_id = class_ID
    self.bp_method_id = method_ID
    self.bp_thread_id = thread_ID
    self.bp_location = loc_index

    if (event_kind == EVENT_METHOD_EXIT_WRV):
      if buf[i] == 86: # "V" void
        ret_val = 0
      elif buf[i] == 90: # "Z" bool
        ret_val = buf[i+1]
      elif buf[i] == 73: # "I" Integer
        ret_val = struct.unpack(">I",buf[i+1:i+5])[0]
      elif buf[i] == 74: # "J" Long
        ret_val = struct.unpack(">Q",buf[i+1:i+9])[0]
      elif buf[i] == 76: # "L" Object
        ret_val = 0 #struct.unpack(">Q",buf[i+1:i+9])[0]
        # TODO: parse classes
        '''
        [JdwpClient] Retval Type: b'L\x00\x00\x00\x00\x00\x00,\x88'
        [JdwpClient] Retval Type: b's\x00\x00\x00\x00\x00\x00-\xb7'
        -> but do we win something if we know that? its probably the classId
        '''
      else:
        dbgPrint("Retval Type: {}".format(buf[i:]))
        # TODO: array reutrn types

  elif (event_kind == EVENT_CLASS_PREPARE):
    refTypeTag      = buf[i]
    referenceTypeID = struct.unpack(">Q",buf[i+1:i+9])[0]
    sig_len         = struct.unpack(">I",buf[i+9:i+13])[0]
    signature       = buf[i+13:i+13+sig_len]
    status          = struct.unpack(">I",buf[i+13+sig_len:i+13+sig_len+4])[0]

  return req_ID, thread_ID, ret_val
