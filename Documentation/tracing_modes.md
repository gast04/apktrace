# apktrace Tracing Modes

## Problem

apktrace currently uses `SUSPEND_EVENT_THREAD` for all event registrations. When tracing hot code paths, the VM continues firing events from other threads while apktrace waits for the `resume_vm()` reply. If more than 1024 events queue up during that window, the connection fails:

```
[apktrace] Failed to resume VM: Too many pending command packets while waiting for reply 11613
```

This document proposes three tracing modes with different suspend policies, each trading off between intrusiveness, feature availability, and reliability.

---

## JDWP Suspend Policies

| Policy | Value | Behavior |
|---|---|---|
| `SUSPEND_NONE` | 0 | VM fires event as a notification, nothing is suspended |
| `SUSPEND_EVENT_THREAD` | 1 | Only the thread that triggered the event is suspended |
| `SUSPEND_ALL` | 2 | All threads in the VM are suspended when any event fires |

---

## Mode 1: Log-Only (`SUSPEND_NONE`)

**CLI flag:** `--log-only`

**Suspend policy:** `SUSPEND_NONE`

### Behavior

The VM sends method entry/exit events as fire-and-forget notifications. No threads are suspended, no `resume_vm()` is needed. apktrace reads events from the socket as fast as it can and logs them.

### Available features

| Feature | Available | Notes |
|---|---|---|
| Method entry/exit logging | Yes | Class name, method name, thread ID |
| Return values (WRV) | Yes | Delivered in the event packet |
| Call depth / indentation | Yes | Tracked client-side by the tracer |
| Accurate timing | No | Processing lag skews timestamps, events may batch |
| Backtraces (`-b`) | No | Thread is running, stack frames are unstable |
| Thread name resolution | Partial | Requires JDWP query — may race but usually works |
| Class/method name resolution | Yes | VM-level queries, not thread-dependent |

### Tradeoffs

- **Fastest mode** — minimal impact on app performance
- **No queue overflow** — no resume round-trip, no pending packet buildup
- **No backtraces** — `ThreadReference.Frames` is unreliable on a running thread
- **No accurate duration** — entry/exit timestamps reflect when apktrace *read* the event, not when it *fired*
- Events may arrive out of order under heavy load

### Implementation changes

- Event registration: change `SUSPEND_EVENT_THREAD` → `SUSPEND_NONE` in `events.rs`
- Main loop: remove the `resume_vm()` call after `wait_for_event()`
- Skip backtrace collection even if `-b` is specified (warn the user)
- `wait_for_event()`: since no reply-waiting happens, `read_buffer()` just reads the next command packet directly — no queue pressure

### Event loop

```
loop {
    event = read_event()        // blocking read, no queue issues
    resolve class/method names  // JDWP queries still work
    log event
    // no resume needed
}
```

---

## Mode 2: Thread-Suspend (`SUSPEND_EVENT_THREAD`)

**CLI flag:** default behavior

**Suspend policy:** `SUSPEND_EVENT_THREAD`

### Behavior

When an event fires, only the triggering thread is suspended. Other threads continue running. apktrace processes the event, inspects the suspended thread's state, then resumes it.

### Available features

| Feature | Available | Notes |
|---|---|---|
| Method entry/exit logging | Yes | |
| Return values (WRV) | Yes | |
| Call depth / indentation | Yes | |
| Accurate timing | Partial | Accurate per-thread, but thread is paused during processing |
| Backtraces (`-b`) | Yes | Thread is suspended, stack frames are stable |
| Thread name resolution | Yes | |
| Class/method name resolution | Yes | |

### Tradeoffs

- **Backtraces work** — the triggering thread is frozen, so stack inspection is reliable
- **Queue overflow risk** — other threads keep firing events during processing + resume round-trip
- **Moderate app impact** — only the traced thread pauses; others run freely
- Duration measurements include the time the thread was suspended (inflated)

### Current issue and mitigations

The queue overflow happens because `resume_vm()` calls `wait_reply()`, which blocks reading packets until the reply arrives. During that window, events from other threads accumulate. Mitigations:

1. **Increase queue limit** — raise `MAX_PENDING_COMMAND_PACKETS` from 1024 to 8192+. Simple but doesn't eliminate the problem.

2. **Fire-and-forget resume** — send the resume command but don't wait for its reply. The reply is just an empty acknowledgment. Handle it when it arrives as part of normal event reading. This eliminates the blocking window entirely.

3. **Resume thread instead of VM** — use `ThreadReference.Resume` (command 11,3) to resume only the suspended thread. Faster round-trip, smaller blast radius.

### Event loop (with fire-and-forget resume)

```
loop {
    event = read_event()
    resolve class/method names
    collect backtrace (if -b)
    log event
    send_resume_vm()            // fire and forget, don't wait for reply
    // reply arrives later, discarded or handled in next read_event()
}
```

---

## Mode 3: Full Suspend (`SUSPEND_ALL`)

**CLI flag:** not implemented

**Suspend policy:** `SUSPEND_ALL`

### Behavior

When any event fires, the entire VM freezes — all threads stop. apktrace processes the event(s) with full access to the VM state, then resumes everything. The VM delivers events as composite packets (multiple events batched into one packet when they fire simultaneously).

### Available features

| Feature | Available | Notes |
|---|---|---|
| Method entry/exit logging | Yes | |
| Return values (WRV) | Yes | |
| Call depth / indentation | Yes | |
| Accurate timing | Yes | Threads frozen → no time passes during processing |
| Backtraces (`-b`) | Yes | All threads frozen, full stack inspection |
| Thread name resolution | Yes | |
| Class/method name resolution | Yes | |
| Cross-thread state inspection | Yes | Can inspect *any* thread, not just the event thread |
| Composite event batching | Yes | Multiple simultaneous events in one packet |

### Tradeoffs

- **No queue overflow** — the VM can't fire new events while suspended, so no packets arrive during processing or resume
- **All JDWP features available** — backtraces, thread inspection, frame variables, everything
- **Most accurate timing** — wall clock doesn't advance for the app during processing
- **Most intrusive** — the app freezes on every event; heavy tracing makes the app noticeably slow
- **Composite events** — the VM may batch events from multiple threads into a single response, which is more efficient but requires processing multiple events per packet (already supported by `parse_event_response`)

### Event loop

```
loop {
    event_batch = read_event()  // may contain multiple events (composite)
    for event in batch:
        resolve class/method names
        collect backtrace (if -b)
        inspect cross-thread state (if needed)
        log event
    resume_vm()                 // safe — no competing events possible
}
```

---

## Comparison Matrix

| | Log-Only | Thread-Suspend | Full Suspend |
|---|---|---|---|
| **Suspend policy** | `SUSPEND_NONE` | `SUSPEND_EVENT_THREAD` | `SUSPEND_ALL` |
| **App performance impact** | Minimal | Moderate | High |
| **Queue overflow risk** | None | High (mitigable) | None |
| **Method logging** | Yes | Yes | Yes |
| **Return values** | Yes | Yes | Yes |
| **Backtraces** | No | Yes (event thread) | Yes (all threads) |
| **Accurate timing** | No | Partial | Yes |
| **Cross-thread inspection** | No | No | Yes |
| **Resume needed** | No | Yes | Yes |
| **Best for** | High-level overview, hot paths | Per-thread debugging | Full analysis, slow methods |

---

## Suggested CLI Interface

```
apktrace <target> --log-only

  --log-only  SUSPEND_NONE — log-only, no backtraces

Default behavior preserves the existing SUSPEND_EVENT_THREAD flow.
```

When `--log-only` is used with `-b` (backtrace), apktrace should warn:

```
[apktrace] Warning: backtraces disabled in log mode (requires thread suspension)
```

---

## Implementation Priority

1. **Add `--log-only` CLI flag** and wire suspend policy through to event registration
2. **Skip `resume_vm()` in log-only mode** — simplest change, immediately fixes overflow for the common case
3. **Fire-and-forget resume for thread mode** — fixes the overflow for thread mode without changing semantics
4. **Full suspend mode** — change suspend policy to `SUSPEND_ALL`, process composite event batches
