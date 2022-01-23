'''
  VM handling commands
'''

from ._protvars import *

def suspendvm(self):
  self.con.sendPacket(SUSPENDVM_SIG)
  self.con.waitReply()

def resumevm(self):
  self.con.sendPacket(RESUMEVM_SIG)
  self.con.waitReply()
