import gdb

# get inferior
inf = gdb.inferiors()[0]

# get name of newly loaded library
mem_loc = gdb.parse_and_eval("*(*((int*)($esp+4))+4)")
mem = inf.read_memory(mem_loc, 256)

# returns type buffer on python2, and memoryview on python3 
# yea thats improvable...
lib_name = ""
for i, b in enumerate(mem):
  if b == '\x00':
    lib_name = mem[:i]
    break

# get load address
mem_loc = gdb.parse_and_eval("*((int*)($esp+4))")
mem = inf.read_memory(mem_loc, 4)
load_addr = ord(mem[0]) | ord(mem[1])<<8 | ord(mem[2])<<16 | ord(mem[3])<<24

print("{} : {}".format(hex(load_addr), lib_name))

