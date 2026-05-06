# apktrace

Trace all method entries and exits of a running Android/Java application via JDWP. On method exit, the duration is printed alongside an indented call tree showing nesting depth. On Ctrl+C, a performance summary is printed.

The app must have `android:debuggable="true"` set and be already running. Attach by PID or package name.

## Usage

```
>> apktrace --help
Attach to a running Android/Java application via JDWP to trace method entry/exit with timing

Usage:
  apktrace <pid|package> [OPTIONS]
  apktrace -l

positional arguments:
  target                Process PID or package name

optional arguments:
  -c, --class <pattern>  Class pattern to trace (e.g., 'com.myapp.*')
  -p, --port <port>      Local TCP port for JDWP forwarding (default: 33333)
  -o, --output <file>    Output file for trace log (default: stdout)
  -b, --backtrace <file> Output file for backtrace log (unique prints only)
  --verbose              Enable verbose output
  -l, --list             List debuggable processes
  -v, --version          Show version
```

## Workflow

1. Start the app on device in debug mode
2. Find the PID (or use package name directly)
3. Run `apktrace`

```
# List all debuggable processes
>> apktrace -l
Debuggable processes:
       22319  com.example.app

# Attach by package name, trace all classes
>> apktrace com.example.app

# Attach by PID, filter to specific package classes only
>> apktrace 22319 -c 'Lcom/example/app/*'

# Save trace to file for later analysis
>> apktrace com.example.app -c 'Lcom/example/app/*' -o trace.out
```

## Output format

Each line shows timestamp, direction (`>>` entry / `<<` exit), thread name and ID, native flag (`N` for native methods), call depth indentation, class, method, duration, and return value (for non-void exits).

```
[00:00:00.000] >> [main:22319]   Lcom/example/app/MainActivity; -> onClick(Landroid/view/View;)V
[00:00:00.004] >> [main:22319]     Lcom/example/app/MainActivity; -> handleClick(Landroid/view/View;)V
[00:00:00.007] >> [main:22319]       Lcom/example/app/PinHandling; -> checkIfPinExists()Z
[00:00:00.009] << [main:22319]       Lcom/example/app/PinHandling; -> checkIfPinExists()Z 2.08ms = 1
[00:00:00.011] << [main:22319]     Lcom/example/app/MainActivity; -> handleClick(Landroid/view/View;)V 7.12ms
[00:00:00.015] << [main:22319]   Lcom/example/app/MainActivity; -> onClick(Landroid/view/View;)V 15.34ms
```

## Performance summary

Press Ctrl+C to stop tracing. A summary is printed showing the top 30 methods by self-time (time in the function itself, excluding callees — equivalent to flamegraph self-time):

```
══════════════════════════════════════════════════════════════════════════════════════════════════════════════════
 Method Timing Summary
══════════════════════════════════════════════════════════════════════════════════════════════════════════════════

Total events processed: 1842
Unique methods traced: 47

  Self%      Calls   Self(ms)  Total(ms)    Avg(us)    Max(us)  Method
──────────────────────────────────────────────────────────────────────────────────────────────────────────────────
  42.3%         12      87.14     157.18       7261      12340  Lcom/example/app/Highwind; -> <clinit>()V
  18.1%        340      37.22      37.22        109        520  Lkotlin/jvm/internal/Intrinsics; -> checkNotNullParameter(...)V
   ...
```

## Trace analysis

The `scripts/analyze_trace.py` script re-analyzes a saved trace file, with additional filtering and sorting options:

```
>> python scripts/analyze_trace.py trace.out
>> python scripts/analyze_trace.py trace.out --sort-by total
>> python scripts/analyze_trace.py trace.out --filter 'com/example/app'
>> python scripts/analyze_trace.py trace.out --ignore ignore.txt
>> python scripts/analyze_trace.py trace.out --ignore-pattern 'Lkotlin/-> checkNotNull'
```

Sort options: `self` (default), `total`, `avg`, `calls`, `max`.

The `ignore.txt` file format:
```
# One pattern per line: ClassName -> methodName substring match
Lkotlin/jvm/internal/Intrinsics; -> checkNotNullParameter
# Or just a substring to match anywhere in "ClassName -> methodName"
-> access$
```

## Prerequisites

- `adb` in PATH
- Device connected with USB debugging enabled
- App built with `android:debuggable="true"`
