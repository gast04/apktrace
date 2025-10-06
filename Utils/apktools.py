import re
import sys
import time

import subprocess as sp

import Utils.definitions as defs
from Utils.clogger import create_logger

logger = create_logger(logger_name="apktools", prefix="APKTOOL")

def startApp(package_id:str, activity: str):

  if defs.STARTUP_CLEAR:
    cmd = ["adb", "shell", "pm", "clear", package_id]
    logger.debug("clear CMD \"{}\"".format(' '.join(cmd)))
    p = sp.Popen(cmd, stdout=sp.PIPE)
    p.wait()

  cmd = ["adb", "shell", "am", "start",
    "-D", "-n", package_id+"/"+activity]
  logger.debug("start CMD \"{}\"".format(' '.join(cmd)))
  p = sp.Popen(cmd, stdout=sp.PIPE)
  p.wait()
  time.sleep(2)

  # TODO: this command will be different on real devices...
  cmd = ["adb", "shell", "ps"]
  p = sp.Popen(cmd, stdout=sp.PIPE)
  # p.wait() this causes hangs on real devices...
  time.sleep(1)
  processes = p.stdout.readlines()

  for proc in processes:
    if package_id in str(proc):
      break

  if len(processes) == 0:
    logger.error("NO emulator or device found!")
    sys.exit(0)

  m = re.search("[\ ]+[0-9]+[\ ]",str(proc))
  defs.PROC_PID = int(m.group(0).strip())

  cmd = ["adb", "forward", "tcp:33333", "jdwp:{}".format(defs.PROC_PID)]
  logger.debug("forward CMD   \"{}\"".format(' '.join(cmd)))
  p = sp.Popen(["adb", "forward", "tcp:33333", "jdwp:{}".format(defs.PROC_PID)])
  p.wait()
