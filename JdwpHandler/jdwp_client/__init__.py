from .Connection import Connection

class JdwpClient(object):

  def __init__(self, host, port):
    # https://python-patterns.guide/gang-of-four/composition-over-inheritance/
    # Bridge Pattern
    self.con = Connection(host, port)
    self.bp_class_id = 0
    self.bp_method_id = 0
    self.bp_thread_id = 0
    self.bp_location = 0

    self.methods = {}
    self.fields = {}
    self.threads = {}

    self.watchlist = []
    return

  from ._client import handshake, start, getVersion
  from ._vm import suspendvm, resumevm
  from ._class import fetchAllClasses, get_class_by_id

  # TODO: event clean up, export in a cleaner way
  from ._events import send_event, parse_event_response, clear_event
  from ._events import clear_events, wait_for_event

  from ._method import send_method_entry, send_method_exit_wrv, send_method_cmd
  from ._method import get_methods_by_id, get_methods

  # TODO, below here not everything has to be exported, create private stuff
  from ._thread import query_thread, suspend_thread, status_thread, resume_thread
  from ._thread import get_thread_by_id, get_thread_by_name, allthreads

  from ._string import readstring, solve_string, createstring, buildstring

  # rework, these should not be exported
  from ._client import idsizes, parse_entries, getBpValues, format
