import Utils.apktools as at
import Utils.utils as uu
import JdwpHandler.jdwp_handler as jh

from Utils.definitions import dbgLog, dbgInfo
import Utils.definitions as defs

# parse arguments
package_name, activity, watchfile = uu.parseCmdArguments()

# clean start application
at.startApp(package_name, activity)

# establish connection
jh.initConnection()

# stop on class prepare
#classname = "com.example.firsttestapp.MainActivity"
#prep_id = jh.breakOnClassPrepare(classname)
#jh.continueDebugging()
#jh.waitForPrepareEvent(prep_id)

# setup class watchlist
watchlist = [
  package_name + "*"  # asterisk to allow all classes defined in package
  #"com.denuvo.*"
]
if watchfile is not None:
  dbgLog("Append watchlist file: {}".format(watchfile))
  with open(watchfile, "r") as f:
    cl_raw = f.readlines()

  # remove trailing \n
  watch_cl = [x[:-1] for x in cl_raw]
  watchlist += watch_cl

dbgLog("Watchlist: {}".format(watchlist))

jh.setClassWatchList(watchlist)

# track all method entries and exits, use watchlist
entry_id = jh.breakOnMethodEntry(True)
exit_id  = jh.breakOnMethodExitWRV(True)

while True:
  jh.continueDebugging()
  class_id, method_id, thread_id, entry_event, is_native = jh.waitForEvent(entry_id, exit_id)

  # only print args on function entry
  if not entry_event:
    continue

  if is_native and defs.NATIVE_STOP:
    dbgInfo("Native Function Entry, continue? (any key)")
    input()

  # TODO: get method arguments
  # -> this is not supposed to be read form jvm
  # get current function args,
  # did we already call the function, and stopped at the entry?
  #slots = jh.variableTableInformation(class_id, method_id)
  
  # fetch values with loaded slots
  #jh.getVarValues(slots, thread_id, class_id, method_id)
