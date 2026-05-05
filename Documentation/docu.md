# File structure

```
src/
  main.rs                                    # CLI entry point, argument parsing, event loop
  Utils/
    apktools.rs                              # ADB utilities (forward, pidof, jdwp list)
  JdwpHandler/
    jdwp_handler.rs                          # Top-level interface for apktrace
    JdwpClient/
      jdwp_client.rs                         # JDWP command orchestration, event dispatch
      protocol_vars.rs                       # JDWP command/event code constants
      events.rs                              # Event packet creation and parsing
      classes.rs                             # Class ID lookup and caching
      methods.rs                             # Method ID lookup and caching
      utils.rs                               # IdSizes, Version, Threads, helpers
      tracer.rs                              # Call stack tracking and timing stats
      Connection/
        connection.rs                        # TCP socket, packet framing (send/receive)
```

### Dependency graph

```
main.rs
 ├── Utils/apktools.rs
 └── JdwpHandler/jdwp_handler.rs
      └── JdwpClient/jdwp_client.rs
           ├── protocol_vars.rs
           ├── events.rs
           ├── classes.rs
           ├── methods.rs
           ├── utils.rs
           ├── tracer.rs
           └── Connection/connection.rs
```

## main.rs

Parses CLI arguments (`argparse`), resolves the target PID (via `apktools` if a package name is given), sets up JDWP forwarding, calls `jdwp_handler::init_connection`, registers method entry/exit events (with optional class pattern filter), then runs the event loop until Ctrl+C. On exit, prints the timing summary.

## Utils/apktools.rs

Thin wrappers around `adb` shell commands:

- `list_debuggable_pids()` — runs `adb jdwp`, resolves each PID to a package name
- `get_pid_by_package(name)` — runs `adb shell pidof <name>`
- `get_package_by_pid(pid)` — reads `/proc/<pid>/cmdline` via adb
- `forward_jdwp(port, pid)` — runs `adb forward tcp:<port> jdwp:<pid>`

## JdwpHandler/jdwp_handler.rs

Public API consumed by `main.rs`. Wraps `JdwpClient` calls into named operations:

- `init_connection` — connect, handshake, suspend VM, fetch ID sizes, version, all classes
- `break_on_method_entry` / `break_on_method_entry_match` — register METHOD_ENTRY event (global or pattern-filtered)
- `break_on_method_exit_wrv` / `break_on_method_exit_match` — register METHOD_EXIT_WITH_RETURN_VALUE event
- `resume_vm` — send RESUMEVM command
- `wait_for_event` — read and dispatch one JDWP event packet
- `set_log_file` / `flush_log` / `print_summary` — delegate to tracer

## JdwpClient/jdwp_client.rs

Owns all JDWP state: connection, class/method/thread caches, registered events, and the tracer. Implements:

- `handshake` — exchanges the `JDWP-Handshake` string
- `suspend_vm` / `resume_vm` — VM-level suspend/resume
- `get_idsizes` / `get_version` / `fetch_classes` — initialization queries
- `evt_entry_class_match` / `evt_entry_class_exclude` — register METHOD_ENTRY events
- `evt_exit_wrv_class_match` / `evt_exit_wrv_class_exclude` — register METHOD_EXIT_WRV events
- `wait_for_event` — parses the event packet, resolves class/method/thread names from cache, formats and logs each entry/exit line with indentation, delegates timing to `tracer`

## JdwpClient/protocol_vars.rs

Constants for JDWP command set signatures (e.g. `VERSION_SIG = (1,1)`), event kind codes (`EVENT_METHOD_ENTRY = 40`, `EVENT_METHOD_EXIT_WRV = 42`), modifier kinds (`MODKIND_CLASS_MATCH`, `MODKIND_CLASS_EXCLUDE`), and suspend policies.

References:
- https://docs.oracle.com/javase/7/docs/platform/jpda/jdwp/jdwp-protocol.html

## JdwpClient/events.rs

Builds JDWP EventRequest.Set packets (with class match or class exclude modifiers) and parses incoming composite event packets into `EventResponse` structs containing thread ID, class ID, method ID, and return value.

## JdwpClient/classes.rs

Fetches all loaded classes from the VM (`AllClasses` command) and caches them by reference type ID. `get_name_by_id` resolves a class ID to its JVM descriptor string (e.g. `Lcom/example/app/MainActivity;`), fetching lazily if not yet cached.

## JdwpClient/methods.rs

Fetches methods for a class (`ReferenceType.Methods` command) and caches them by `(class_id, method_id)`. Each `Method` record stores name, JVM signature, native flag, and whether the return type is void.

## JdwpClient/utils.rs

Parses `IdSizes` and `Version` reply packets. Provides `Threads` cache (thread ID → name), `get_thread_by_id` (issues `ThreadReference.Name`), and `get_current_time` for log timestamps.

## JdwpClient/tracer.rs

Tracks per-thread call stacks to compute self-time (time in a function excluding its callees, equivalent to flamegraph self-time) and total-time. On `method_entry`, pushes a `MethodCall` onto the thread's stack. On `method_exit`, pops it, computes duration, subtracts accumulated child time, and updates `MethodStats`. `print_summary` renders the top 30 methods sorted by self-time.

Also handles optional file logging: when an output file is set, trace lines go to a buffered file writer instead of stdout.

## JdwpClient/Connection/connection.rs

Manages the TCP socket. Builds JDWP command packets (11-byte header + data), assigns incrementing packet IDs, sends raw bytes, and reads reply/event packets. `read_buffer` blocks until a full packet arrives. `read_reply_buffer` waits for the reply matching a specific packet ID.
