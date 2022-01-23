import gdb

print("running gdb python script...")
print("gdb version: {}".format(gdb.VERSION))

test = gdb.execute("info shared", False, True)
print(type(test))
gdb.flush()
lines = test.split("\n")

#for line in lines:
#  print("[*] " + line)

for b in gdb.breakpoints():
  print(b.location)
  print(b.hit_count)
  print(b.visible)
  print(b.type)
  print(b.commands)

test = gdb.execute("x/s *(*((int*)($esp+4))+4)", False, True)
print("|" + test.strip() + "|")

test = gdb.execute("x/xw *((int*)($esp+4))", False, True)
print("|" + test.strip() + "|")

'''
struct link_map {
  void *l_addr;
  char *l_name;
  void *l_ld;
  struct link_map *l_next;
  struct link_map *l_prev;
}
'''

# get string start of newly loaded library
mem_loc = gdb.parse_and_eval("*(*((int*)($esp+4))+4)")
inf = gdb.inferiors()[0]
mem = inf.read_memory(mem_loc, 256)
# returns type buffer on python2, and memoryview on python3 

for i, b in enumerate(mem):
  if b == '\x00':
    print(mem[:i])
    break

# print load address
mem_loc = gdb.parse_and_eval("*((int*)($esp+4))")
mem = inf.read_memory(mem_loc, 4)
print("mem[2]: {}".format(ord(mem[2])))
load_addr = ord(mem[0]) | ord(mem[1])<<8 | ord(mem[2])<<16 | ord(mem[3])<<24
print(hex(load_addr))

'''
(gdb) x/xw *((int*)($esp+4))
0xf47841ac:	0xc4a23000

shows the base address, 
info shared shows the address of the text section
'''


print(gdb.solib_name(0xc5a53350))
# only works if gdb has the file loaded, meaning when "info shared" shows
# the addresses, but with the breakpoint we would get all load addresses
# and would theoretically know more than gdb

inp = raw_input("Hit enter to continue...")
print(inp)

if inp.strip() == 'x' or inp.strip() == 'X':
  # set new breakpoint at __dl_notify_gdb_of_load+1
  # depends on arch, and instructions size
  # +1 for x86 
  # +4 for arm 
  '''
    or simply parse it by letting gdb do the dissassembly,
    and then parse the address

    (gdb) x/2i 0xf490b080
    0xf490b080 <__dl_notify_gdb_of_load>:	  push   %ebp
    0xf490b081 <__dl_notify_gdb_of_load+1>:	mov    %esp,%ebp
  '''

  gdb.execute("b* __dl_notify_gdb_of_load+1")
  # this breakpoint has no commands, so we return to the 
  # gdb shell

