# file structure

```
jdwp_handler.py         # top interface for applications
jdwp_client.py          # JDWP implementation (Package creation...)
jdwp_utils.py           # utils for jdwp_client.py
jdpw_protvars.py        # JDWP message signature definition
```

### Dependency Graph:
```
jdwp_handler.py
 └── jdwp_client.py
      ├── jdwp_utils.py
      └── jdwp_protvars.py
```

## jdwp_handler.py

Handles connection to debug server, wraps single 
jdwp_client commands to a single usable command, for
example set breakpoint or setup class event

## jdwp_client.py
create and send packages to server


