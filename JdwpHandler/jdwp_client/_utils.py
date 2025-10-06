import struct
from typing import List, Tuple
from termcolor import colored

from ._protvars import *

version_formats:List[Tuple[str, str]] = [ 
  ('S', "description"), ('I', "jdwpMajor"), ('I', "jdwpMinor"),
  ('S', "vmVersion"), ('S', "vmName")
]

def dbgPrint(msg):
  if isinstance(msg, bytes):
    msg = msg.decode("utf-8")
  print(colored("[JdwpClient] ", "yellow") + msg)

def dbgError(msg):
  if isinstance(msg, bytes):
    msg = msg.decode("utf-8")
  print(colored("[JdwpClient] ERROR: ", "red") + msg)
