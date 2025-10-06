import struct # TODO: remove, move all struct lines to the client
import datetime
from termcolor import colored
from typing import List

from JdwpHandler.jdwp_client import JdwpClient
#from JdwpHandler.jdwp_protvars import *
#import JdwpHandler.jdwp_utils as jdwp_utils


from Utils.clogger import create_logger

logger = create_logger(logger_name="JDWP", prefix=colored("JDWP", 'yellow'))

jdwp_cli = None
init_con: bool = False
debug_mode: bool = True

# print each thread in a different color for readability
THREAD_COLORS = [
  "blue",
  "yellow",
  "magenta",
  "cyan",
  "grey"
]
color_id = 0
thread_colors = {}

def initConnection(target: str = "127.0.0.1", port: int = 33333):
  global jdwp_cli, init_con
  if init_con:
    logger.debug("Already connected!")
    return

  jdwp_cli = JdwpClient(target, port)
  jdwp_cli.start()
  init_con = True

def breakOnMethodEntry(use_watch=True):
  if not init_con:
    logger.debug("NOT connected!")
    return

  # create method entry break event
  return jdwp_cli.send_method_entry(use_watch)

def breakOnMethodExitWRV(use_watch=True):
  if not init_con:
    logger.debug("NOT connected!")
    return

  # create method exit with return value event
  return jdwp_cli.send_method_exit_wrv(use_watch)

def breakOnClassPrepare(classname: str):
  if not init_con:
    logger.debug("NOT connected!")
    return

  # create class prepare event
  return jdwp_cli.send_class_prepare(classname)

def continueDebugging():
  # resume vm and wait for CLASS_PREPARE event
  jdwp_cli.resumevm()

def waitForPrepareEvent(prep_id: int) -> bool:
  # wait for event
  buf = jdwp_cli.wait_for_event()
  req_id, thread_id, ret_val = jdwp_cli.parse_event_response(buf)
  return req_id == prep_id

def waitForEvent(entry_id: int, exit_id: int):
  # wait for event
  buf = jdwp_cli.wait_for_event()
  req_id, thread_id, ret_val = jdwp_cli.parse_event_response(buf)
  # logger.debug("Request ID parsed: " + str(req_id))

  # fetch stop values, TODO: convert location in class type
  class_id, method_id, thread_id, location = jdwp_cli.getBpValues()

  # print stats
  bp_class = jdwp_cli.get_class_by_id(class_id)
  bp_method = jdwp_cli.get_methods_by_id(class_id, method_id)
  if bp_method is None:
    # idk why this happens
    bp_method = {}
    bp_method['name'] = b"unkown"
    bp_method['signature'] = b"()V"
    bp_method['modBits'] = 0

  thread_name = jdwp_cli.get_thread_by_id(thread_id).str() # TODO: caching

  # TODO: add check which response we parsed
  if req_id in entry_id:
    printMethodEntry(thread_id, thread_name, bp_class['signature'].str(),
      bp_method['name'].str() + bp_method['signature'].str(), location, 
      bp_method['modBits'])
  else:
    printMethodExit(thread_id, thread_name, bp_class['signature'].str(),
      bp_method['name'].str() + bp_method['signature'].str(),
      location, bp_method['modBits'], ret_val)

  is_native = bp_method['modBits'] & 0x100 > 0

  return class_id, method_id, thread_id, req_id in entry_id, is_native

def getThreadString(tid, tname):
  global color_id

  color = ""
  if tname in thread_colors:
    color = thread_colors[tname]
  else:
    thread_colors[tname] = THREAD_COLORS[color_id % len(THREAD_COLORS)]
    color = thread_colors[tname]
    color_id += 1
  
  return colored("Thread: {} [{}], ".format(tid, tname), color)

def getFormattedDate():
  return datetime.datetime.now().strftime("%H:%M:%S-%f")

def printMethodEntry(tid, tname, cname, mnamesig, location, modbits):
  msg  = "[" + getFormattedDate() + "] "
  msg += colored("Method Entry, ", "green")
  msg += getThreadString(tid, tname)
  if modbits & 0x100 > 0: # ACC_NATIVE
    msg += colored(cname + " -> ", "yellow")
    msg += colored(mnamesig, "yellow")
  else:
    msg += cname + " -> "
    msg += mnamesig# + ", "
  #msg += colored("Loc: {}".format(str(location)), "cyan")
  print(msg)

def printMethodExit(tid, tname, cname, mnamesig, location, modbits, ret_val):
  msg  = "[" + getFormattedDate() + "] "
  msg += colored("Method Exit,  ", "red")
  msg += getThreadString(tid, tname)
  if modbits & 0x100 > 0: # ACC_NATIVE
    msg += colored(cname + " -> ", "yellow")
    msg += colored(mnamesig + ", ", "yellow")
  else:
    msg += cname + " -> "
    msg += mnamesig + ", "
  #msg += colored("Loc: {}, ".format(str(location)), "cyan")
  msg += colored("Retval: {}".format(ret_val), "magenta")
  print(msg)

def setClassWatchList(watchlist: List[str]):
  jdwp_cli.watchlist = watchlist

# TODO: rework below here
################################################################################

def variableTableInformation(class_id, method_id):
  data = struct.pack(">Q", class_id) + struct.pack(">I", method_id)
  slots = jdwp_cli.send_method_cmd(VARIABLE_TABLE_GEN_SIG, data)
  return slots

def getVarValues(slots, thread_id, class_id, method_id):
  # TODO: rewrite and move logic to jdwp client

  if len(slots) == 0:
    return

  # get frameID(s), as its needed to get the locals
  data  = struct.pack(">Q", thread_id)
  data += struct.pack(">I", 0)
  data += struct.pack(">I", 0xffffffff)
  frames = jdwp_cli.send_thread_cmd(THREAD_FRAMES_SIG, data)
  '''
    gets all frames of the current thread, sorted by function in class
  '''

  frameID = 0
  for frame in frames:
    if frame[3] == method_id:
      frameID = frame[0]
      break

  # fetch slots of frame
  data  = struct.pack(">Q", thread_id)
  data += struct.pack(">Q", frameID)

  slot_data = b""
  match_slots = 0
  for slot in slots:

    if slot[-1] >= len(slots):
      continue

    slot_data += struct.pack(">I", match_slots) # slot[-1])

    slot_type = slot[2]
    if len(slot_type) != 1:
      slot_type = b"L"

    slot_data += slot_type
    match_slots += 1
    if match_slots == 2:
      break
  
  # if slot index is greater then num slots, it does not work...

  data += struct.pack(">I", match_slots)
  data += slot_data

  jdwp_cli.send_stack_cmd(SF_GETVALUES_SIG, data, False)
  '''
  could be problem of the method is not called yet and we are still in the caller
  not in the callee, 

  try the single step event and after fetch the variables, that will also 
  show if we are already in the function

  but why do we have the frameID if the function is not executed yet?
  '''

def singleStep(cli, thread_id, step_size, depth_size):
  data = [
    (MODKIND_STEP,
    struct.pack(">Q", thread_id),         # thread_id (in which to do the step)
    struct.pack(">I", step_size),         # StepSize 1 == Step to next source line
    struct.pack(">I", depth_size)),       # StepDepth 1 == Step Over any method calls

    (MODKIND_COUNT,
    struct.pack(">I", 1))                 # count of how often to execute/hit it
  ]

  # TODO: merge to get the structure:
  '''
    (MODKIND, data)
  '''

  cpId = cli.send_event( EVENT_SINGLE_STEP, *data )
  print("[+] Created SINGLE_STEP EVENT id=%x" % cpId)

  cli.resumevm()

  buf = cli.wait_for_event()
  req_ID, thread_ID, ret_val = cli.parse_event_response(buf, cpId)

  # retrieve step values
  bp_class_id, bp_method_id, bp_thread_id, bp_location = cli.getBpValues()

  # print step stats
  bp_class = cli.get_class_by_id(bp_class_id)
  bp_method = cli.get_methods_by_id(bp_class_id, bp_method_id)

  print("Single Step:")
  print("   thread: " + str(bp_thread_id))
  print("    class: " + bp_class['signature'].str())
  print("   method: " + bp_method['name'].str() + " | " + bp_method['signature'].str())
  print(" location: " + str(bp_location))

  return bp_thread_id
