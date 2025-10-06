import logging
import argparse
from typing import Tuple
from termcolor import colored

import Utils.definitions as defs

logger = logging.Logger(name="Utils")

def parseCmdArguments():
      parser = argparse.ArgumentParser(description='Trace APK files easily')

      # positional arguments (required)
      parser.add_argument("package_name", type=str,
            help='Package Name used to start Application')
      parser.add_argument("activity", type=str,
            help='start activity of the Application')

      # optional arguments
      parser.add_argument("-w", "--watchlist",  type=str, metavar="<filename>",
            dest="watch_list_file",
            default=None,
            help="File containing classes to watch, (class per line)")

      parser.add_argument("-c", "--clear", dest="clear_apk", default=False,
            help='Clear APK before start', action="store_true")

      parser.add_argument("-d", "--debug", dest="debug_mode", default=False,
            help='Verbose mode', action="store_true")

      parser.add_argument("-n", "--native", dest="native_stop", default=False,
            help='Break on native methods', action="store_true")

      parser.add_argument("--version", dest="version", default=False,
            help='Print apktrace version', action="store_true")

      args = parser.parse_args()

      if args.version:
            logger.info(colored("\n    apktrace Version: ", "green") +
            colored(defs.APKTRACE_VERSION, "blue", attrs=["bold"]))
            logger.info(colored("        from ","yellow") + colored("and ","cyan") +
            colored("with ","magenta") + colored("niku", "red"))
            logger.info("")
            return None

      defs.DEBUG_MODE = args.debug_mode
      defs.STARTUP_CLEAR = args.clear_apk
      defs.NATIVE_STOP = args.native_stop

      if args.package_name == None or args.activity == None:
            parser.print_help()
            return None

      return args.package_name, args.activity, args.watch_list_file
