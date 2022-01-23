import struct
from ._protvars import *

def fetchAllClasses(self):
  self.con.sendPacket(ALLCLASSES_SIG)
  buf = self.con.readReplyBuf()
  formats = [ ('C', "refTypeTag"),
              (self.referenceTypeIDSize, "refTypeId"),
              ('S', "signature"),
              ('I', "status")
            ]
  classes_tmp = self.parse_entries(buf, formats)

  # restructure classes to dictionary with refTypeId key
  self.classes = {}
  for cl in classes_tmp:
    self.classes[cl['refTypeId']] = cl
  return

def send_class_prepare(self, classname):
  data = [
      (MODKIND_CLASS_MATCH, struct.pack(">I", len(classname)) + classname.bytes()),
      (MODKIND_COUNT, struct.pack(">I", 1))]
  return self.send_event( EVENT_CLASS_PREPARE, *data )

def get_class_by_name(self, name):
  for k in self.classes:
    if self.classes[k]["signature"].lower() == name.lower().bytes():
      return entry
  
  # TODO: implement reload
  dbgPrint("Could not find Class with Name: {}".format(name))
  return None

def get_class_by_id(self, refTypeId):

  if refTypeId in self.classes:
    return self.classes[refTypeId]

  # reload classes, maybe new one got loaded
  self.fetchAllClasses()
  if refTypeId in self.classes:
    return self.classes[refTypeId]
  else:
    dbgPrint("Could not find Class with refTypeId: {}".format(refTypeId))
    return None

