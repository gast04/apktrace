from termcolor import colored

APKTRACE_VERSION = "2.0.0"
DEBUG_MODE = False
NATIVE_STOP = False
PROC_PID = 0

def dbgInfo(msg: str):
  print(colored("[apktrace] ", "green") + "INFO  : " + msg)

def dbgLog(msg: str):
  if not DEBUG_MODE:
    return
  print(colored("[apktrace] ", "green") + "DEBUG : " + msg)

def dbgError(msg: str):
  print(colored("[apktrace] ", "green") + "ERROR : " + msg)
