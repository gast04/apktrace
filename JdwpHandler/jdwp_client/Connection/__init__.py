import socket

class Connection(object):
  # Handles packaging and sending/receiving
  def __init__(self, host, port):
    self.host = host
    self.port = port
    self.id = 0x01
    self.socket = None

  def connect(self):
    self.socket = socket.socket()
    try:
      self.socket.connect((self.host, self.port))
    except socket.error as msg:
      raise EnvironmentError(f"Could not connect: {msg}")

  def close(self):
    self.socket.close()

  from .packaging import rawSend, sendPacket
  from .packaging import rawRead, waitReply
  from .packaging import readReplyBuf, readReplyReqId, readReplyString
