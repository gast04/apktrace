# How JDWP works

JDWP (Java Debug Wire Protocol) is the lowest layer of JPDA (Java Platform Debugger Architecture). It defines the binary wire format a debugger uses to communicate with a Java/Dalvik VM. apktrace acts as a JDWP client — it speaks the protocol directly over TCP, without relying on JDI or jdb.

Spec references:
- https://docs.oracle.com/en/java/javase/15/docs/specs/jdwp/jdwp-spec.html
- https://docs.oracle.com/javase/7/docs/platform/jpda/jdwp/jdwp-protocol.html

## Transport: ADB forwarding

The Android VM exposes a JDWP transport per debuggable process, identified by PID. `adb forward tcp:<port> jdwp:<pid>` creates a local TCP tunnel so the debugger can connect to `127.0.0.1:<port>`.

```
  Host                         Device
  ───────                      ───────
  apktrace ←→ TCP:33333 ←→ adb ←→ jdwp:<pid> ←→ ART VM
```

## Handshake

Before any JDWP packets, both sides exchange the ASCII string `JDWP-Handshake` (14 bytes). The client sends it, the VM echoes it back. If the echo doesn't match, the connection is rejected.

## Packet format

Every JDWP packet has an 11-byte header. There are two packet types — **command packets** (sent by client or VM) and **reply packets** (response to a command):

```
Command packet:
  ┌──────────────────┬──────────────┬───────┬─────────────┬─────────┬──────┐
  │ length (4 bytes)  │ id (4 bytes) │ flags │ command set │ command │ data │
  │     u32 BE        │   u32 BE     │  0x00 │    u8       │   u8    │ ...  │
  └──────────────────┴──────────────┴───────┴─────────────┴─────────┴──────┘

Reply packet:
  ┌──────────────────┬──────────────┬───────┬────────────────┬──────┐
  │ length (4 bytes)  │ id (4 bytes) │ flags │ error (2 bytes)│ data │
  │     u32 BE        │   u32 BE     │  0x80 │    u16 BE      │ ...  │
  └──────────────────┴──────────────┴───────┴────────────────┴──────┘
```

- **length**: total packet size including the 11-byte header
- **id**: unique per packet, the reply uses the same id as the command it answers
- **flags**: `0x00` for commands, `0x80` for replies
- **command set + command**: identifies the operation (e.g. `(1,1)` = VirtualMachine.Version)
- **error**: `0` on success, non-zero is a JDWP error code

All integers are big-endian. Strings are length-prefixed (4-byte u32 length + UTF-8 bytes, no null terminator).

## Variable ID sizes

ID fields (object IDs, reference type IDs, method IDs, field IDs, frame IDs) have **variable sizes** that differ between VMs. Before any command that uses IDs, the client must query `VirtualMachine.IDSizes (1,7)` to learn the byte width of each:

```
IDSizes reply:
  fieldIDSize         (u32) — typically 4 or 8
  methodIDSize        (u32)
  objectIDSize        (u32)
  referenceTypeIDSize (u32)
  frameIDSize         (u32)
```

apktrace stores these in `IdSizes` and uses them to correctly parse/build all subsequent packets. Getting this wrong means every ID field after the first variable-width one is misaligned, corrupting the rest of the packet.

## Initialization sequence

After the handshake, apktrace runs:

```
1. SuspendVM          (1,8)  — freeze all threads to query safely
2. IDSizes            (1,7)  — learn variable ID widths
3. Version            (1,1)  — get VM description, JDWP version
4. AllClasses         (1,3)  — fetch all loaded class signatures + IDs
```

At this point apktrace knows every loaded class and can set up events.

## Command sets used

| Signature | Command                     | Purpose                                  |
|-----------|-----------------------------|------------------------------------------|
| (1,1)     | VirtualMachine.Version      | VM description, JDWP major/minor version |
| (1,3)     | VirtualMachine.AllClasses   | List all loaded classes with type IDs    |
| (1,7)     | VirtualMachine.IDSizes      | Variable ID field widths                 |
| (1,8)     | VirtualMachine.Suspend      | Suspend all threads                      |
| (1,9)     | VirtualMachine.Resume       | Resume all threads                       |
| (2,5)     | ReferenceType.Methods       | List methods of a class by type ID       |
| (11,1)    | ThreadReference.Name        | Get thread name from thread ID           |
| (15,1)    | EventRequest.Set            | Register an event (entry/exit/etc)       |

## Event registration

To receive method events, apktrace sends `EventRequest.Set (15,1)` packets. The packet body has:

```
EventRequest.Set data:
  eventKind      (u8)  — what to watch (40 = METHOD_ENTRY, 42 = METHOD_EXIT_WITH_RETURN_VALUE)
  suspendPolicy  (u8)  — what to freeze on event (1 = SUSPEND_EVENT_THREAD)
  modifierCount  (u32) — number of filter modifiers
  modifiers[]          — each starts with modKind (u8), followed by modifier-specific data
```

apktrace uses two modifier kinds:

- **ClassMatch (5)**: followed by a string pattern (e.g. `com.myapp.*`). Only events from matching classes are delivered. Used with `-c` flag.
- **ClassExclude (6)**: followed by a string pattern. Events from matching classes are suppressed. Used in default mode to filter out framework noise (`java.*`, `android.*`, `dalvik.system.*`, etc).

The VM replies with a **request ID** (u32). This ID appears in every delivered event, so the client can match events back to the registration.

Multiple modifiers on a single event request are AND-combined — all must match for the event to fire.

**Suspend policy**: apktrace uses `SUSPEND_EVENT_THREAD (1)`, which freezes only the thread that triggered the event. The client can then inspect state (resolve class names, method names) and must explicitly resume the VM with `VirtualMachine.Resume (1,9)` when done.

## Event delivery

Once events are registered and the VM is resumed, the VM sends **command packets** (not replies) to the client when events fire. These are Composite event packets:

```
Composite event data:
  suspendPolicy  (u8)
  eventCount     (u32)
  events[] — each event:
    eventKind    (u8)
    requestID    (u32)
    threadID     (objectIDSize bytes)
    location:
      typeTag    (u8)        — 1=class, 2=interface
      classID    (referenceTypeIDSize bytes)
      methodID   (methodIDSize bytes)
      index      (u64)       — bytecode offset within method
    [returnValue] — only for METHOD_EXIT_WITH_RETURN_VALUE (kind 42)
```

**The event loop** is:
1. Block on `read_buffer()` — wait for the next command packet from the VM
2. Parse the composite event: extract event kind, request ID, thread/class/method IDs
3. Match the request ID to a registered event to determine the kind
4. Resolve class name (from cache or re-fetch `AllClasses`) and method name (from cache or fetch `ReferenceType.Methods`)
5. Resolve thread name (from cache or query `ThreadReference.Name`)
6. Log the entry/exit line with timing
7. Send `VirtualMachine.Resume (1,9)` to unfreeze the suspended thread
8. Repeat

## Return value parsing (METHOD_EXIT_WITH_RETURN_VALUE)

The return value in a METHOD_EXIT_WRV event is tagged with a JVM type byte:

| Tag byte | ASCII | Type    | Value size      |
|----------|-------|---------|-----------------|
| 66       | 'B'   | byte    | 1 byte          |
| 90       | 'Z'   | boolean | 1 byte          |
| 67       | 'C'   | char    | 2 bytes         |
| 83       | 'S'   | short   | 2 bytes         |
| 73       | 'I'   | int     | 4 bytes         |
| 70       | 'F'   | float   | 4 bytes         |
| 74       | 'J'   | long    | 8 bytes         |
| 68       | 'D'   | double  | 8 bytes         |
| 76       | 'L'   | object  | objectIDSize    |
| 91       | '['   | array   | objectIDSize    |
| 115      | 's'   | string  | objectIDSize    |
| 116      | 't'   | thread  | objectIDSize    |
| 103      | 'g'   | threadgroup | objectIDSize |
| 108      | 'l'   | classloader | objectIDSize |
| 99       | 'c'   | classobj | objectIDSize   |
| 86       | 'V'   | void    | 0 bytes         |

For primitive types, apktrace reads the raw value and displays it. For object types, it shows the object ID. For void, nothing is shown.

## Command packet ordering

The VM can send event command packets at any time — including while the client is waiting for a reply to a command it sent. apktrace handles this by queuing unsolicited command packets in a `pending_command_packets` deque when `read_expected_reply()` encounters them. The next call to `read_buffer()` drains the queue before reading from the socket.

---

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
