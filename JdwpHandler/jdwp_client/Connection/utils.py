from termcolor import colored

def dbgPrint(msg):
  print(colored("[Connection] ", "magenta") + msg)

def dbgError(msg):
  print(colored("[Connection] ERROR: ", "red") + msg)
