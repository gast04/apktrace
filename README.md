# apktrace

Trace all method entries and exits, the exit also prints the return value, if
it is of basic type. The apk must have set the `android:debuggable="true"` flag.

By default it will trace all functions which match(prefixed) `package_name`.

[![asciicast](https://asciinema.org/a/383511.svg)](https://asciinema.org/a/383511)

# Updates
(07-02-2020 add native function highlighting)

# Usage
```
>> python apktrace.py --help
usage: apktrace.py [-h] [-w <filename>] [-c] [-d] [--version]
                   package_name activity

Trace APK files easily

positional arguments:
  package_name          Package Name used to start Application
  activity              start activity of the Application

optional arguments:
  -h, --help            show this help message and exit
  -w <filename>, --watchlist <filename>
                        File containing classes to watch, (class per line)
  -c, --clear           Clear APK before start
  -d, --debug           Verbose mode
  -n, --native          Break on native method entry
  --version             Print apktrace version
```

In action:
```
>> python apktrace.py -d com.example.firsttestapp .MainActivity
[apktrace] LOG   : start CMD "adb shell am start -D -n com.example.firsttestapp/.MainActivity"
[apktrace] LOG   : forward CMD   "adb forward tcp:33333 jdwp:14856"
[00:12:33-057312] Method Entry, Thread: 10635 [main], Lcom/example/firsttestapp/MainActivity; -> <init>()V, 
[00:12:33-103480] Method Exit,  Thread: 10635 [main], Lcom/example/firsttestapp/MainActivity; -> <init>()V, Retval: 0
[00:12:33-125597] Method Entry, Thread: 10635 [main], Lcom/example/firsttestapp/MainActivity; -> onCreate(Landroid/os/Bundle;)V, 
[00:12:33-174525] Method Entry, Thread: 10635 [main], Lcom/example/firsttestapp/MainActivity; -> calcOffset(IILjava/lang/String;)I, 
[00:12:33-175951] Method Exit,  Thread: 10635 [main], Lcom/example/firsttestapp/MainActivity; -> calcOffset(IILjava/lang/String;)I, Retval: 325
[00:12:33-224984] Method Entry, Thread: 10635 [main], Lcom/example/firsttestapp/PinHandling; -> <init>(Ljava/io/File;)V, 
[00:12:33-226337] Method Exit,  Thread: 10635 [main], Lcom/example/firsttestapp/PinHandling; -> <init>(Ljava/io/File;)V, Retval: 0
[00:12:33-227446] Method Entry, Thread: 10635 [main], Lcom/example/firsttestapp/PinHandling; -> checkIfPinExists()Z, 
[00:12:33-230958] Method Exit,  Thread: 10635 [main], Lcom/example/firsttestapp/PinHandling; -> checkIfPinExists()Z, Retval: 1
[00:12:34-578716] Method Exit,  Thread: 10635 [main], Lcom/example/firsttestapp/MainActivity; -> onCreate(Landroid/os/Bundle;)V, Retval: 0
```

# TODO

* there is a known Bug in the methodID size in the Rust implementation
  I wonder how this ever worked^^
* move JdwpHandler in its own repository and use it as a git submodule, to
allow easier usage also for other repositories, for example jdb++
* implement the `--watchlist` argument (not possible see issue)
