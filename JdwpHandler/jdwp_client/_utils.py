import struct
from typing import List, Tuple
from termcolor import colored

from ._protvars import *

version_formats:List[Tuple[str, str]] = [ 
  ('S', "description"), ('I', "jdwpMajor"), ('I', "jdwpMinor"),
  ('S', "vmVersion"), ('S', "vmName")
]

def dbgPrint(msg):
  print(colored("[JdwpClient] ", "yellow") + msg)

def dbgError(msg):
  print(colored("[JdwpClient] ERROR: ", "red") + msg)
