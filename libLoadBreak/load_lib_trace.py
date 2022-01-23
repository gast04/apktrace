import pwn

gdb_path = "/home/niku/Android/NDKs/android-ndk-r21d/prebuilt/linux-x86_64/bin/gdb"

# cmd = [gdb_path ,"--eval-command=\"set auto-solib-add on\"", "--eval-command=\"set solib-search-path /home/niku/Project/GdbAndroid/demo3/lib/x86\"", "--eval-command=\"set sysroot /home/niku/Project/GdbAndroid/android_libs/\"", "--eval-command=\"target remote localhost:12345\""]


p = pwn.process([gdb_path,"-q"])
p.read()  # read gdb startup header

p.sendline("set auto-solib-add on") # thats default anyhow
p.read()

p.sendline("set solib-search-path /home/niku/Project/GdbAndroid/demo3/lib/x86")
print("1: {}".format(p.read()))

p.sendline("set sysroot /home/niku/Project/GdbAndroid/android_libs/")
print("2: {}".format(p.read()))

# finally connect to remote
p.sendline("target remote localhost:12345")
print("3: {}".format(p.read()))

# load symbols of linker binary, to allow breakpoint creation
p.sendline("sharedlibrary linker")
print("4: {}".format(p.read()))

p.sendline("b* __dl_notify_gdb_of_load")
print("5: {}".format(p.read()))

p.sendline("continue")
print("6: {}".format(p.read())) # blocking, waits until bp hit

while True:
  p.sendline("x/s *(*((int*)($esp+4))+4)")
  data_raw = p.read()
  tmp = data_raw.split(b"\t")
  print(tmp)
  if len(tmp) > 1:
    # not the case on startup
    loaded_lib = tmp[1].strip()[1:-1].decode("utf-8")
    print("Loaded: {}".format(loaded_lib))

  p.sendline("continue")
  p.read()
  #print("8: {}".format(p.read())) # blocking, waits until bp hit


