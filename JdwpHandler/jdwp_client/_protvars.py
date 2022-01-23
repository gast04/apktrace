'''
  Documentation:
  * https://docs.oracle.com/javase/7/docs/platform/jpda/jdwp/jdwp-protocol.html
  * https://docs.oracle.com/en/java/javase/15/docs/specs/jdwp/jdwp-spec.html

  * http://pallergabor.uw.hu/androidblog/dalvik_opcodes.html

  Location Definition:

  An executable location. The location is identified by one byte type tag
  followed by a a classID followed by a methodID followed by an unsigned
  eight-byte index, which identifies the location within the method. See below
  for details on the location index. The type tag is necessary to identify
  whether location's classID identifies a class or an interface. Almost all
  locations are within classes, but it is possible to have executable code in
  the static initializer of an interface.
'''

################################################################################
# JDWP protocol variables
################################################################################


HANDSHAKE                 = b"JDWP-Handshake"

REQUEST_PACKET_TYPE       = 0x00
REPLY_PACKET_TYPE         = 0x80

# Command signatures
VERSION_SIG               = (1, 1)
CLASSESBYSIGNATURE_SIG    = (1, 2)
ALLCLASSES_SIG            = (1, 3)
ALLTHREADS_SIG            = (1, 4)
IDSIZES_SIG               = (1, 7)
SUSPENDVM_SIG             = (1, 8)
RESUMEVM_SIG              = (1, 9)
CREATESTRING_SIG          = (1, 11)
CAPABILITIES_SIG          = (1, 12)
CAPABILITIESNEW_SIG       = (1, 17)

SIGNATURE_SIG             = (2, 1)
FIELDS_SIG                = (2, 4)
METHODS_SIG               = (2, 5)
GETVALUES_SIG             = (2, 6)
CLASSOBJECT_SIG           = (2, 11)

INVOKESTATICMETHOD_SIG    = (3, 3)

LINE_TABLE_SIG            = (6, 1)
VARIABLE_TABLE_SIG        = (6, 2)
BYTECODES_SIG             = (6, 3)
VARIABLE_TABLE_GEN_SIG    = (6, 5)

REFERENCETYPE_SIG         = (9, 1)
INVOKEMETHOD_SIG          = (9, 6)
STRINGVALUE_SIG           = (10, 1)
THREADNAME_SIG            = (11, 1)
THREADSUSPEND_SIG         = (11, 2)
THREADRESUME_SIG          = (11, 3)
THREADSTATUS_SIG          = (11, 4)
THREAD_FRAMES_SIG         = (11, 6)
EVENTSET_SIG              = (15, 1)
EVENTCLEAR_SIG            = (15, 2)
EVENTCLEARALL_SIG         = (15, 3)

SF_GETVALUES_SIG          = (16, 1)
SF_SETVALUES_SIG          = (16, 2)

# Event Request Codes
EVENT_SINGLE_STEP         = 1
EVENT_BREAKPOINT          = 2
EVENT_CLASS_PREPARE       = 8
EVENT_CLASS_LOAD          = 10
EVENT_METHOD_ENTRY        = 40
EVENT_METHOD_EXIT         = 41
EVENT_METHOD_EXIT_WRV     = 42 # WRV = WITH_RETURN_VALUE

# other codes
MODKIND_COUNT             = 1
MODKIND_THREADONLY        = 2
MODKIND_CLASS_MATCH       = 5
MODKIND_CLASS_EXCLUDE     = 6
MODKIND_LOCATIONONLY      = 7
MODKIND_STEP              = 10

SUSPEND_EVENTTHREAD       = 1
SUSPEND_ALL               = 2
NOT_IMPLEMENTED           = 99
VM_DEAD                   = 112
INVOKE_SINGLE_THREADED    = 2
TAG_OBJECT                = 76
TAG_STRING                = 115
TYPE_CLASS                = 1

# ERROR values
INVALID_SLOT              = 35
INVALID_EVENT_TYPE        = 102
NATIVE_METHOD             = 511


################################################################################
# JDWP Method modBit (access and property flags)
################################################################################

ACC_PUBLIC                = 0x1
ACC_PRIVATE               = 0x2
ACC_PROTECTED             = 0x4
ACC_STATIC                = 0x8
ACC_FINAL                 = 0x10
ACC_SYNCHRONIZED          = 0x20
ACC_BRIDGE                = 0x40
ACC_VARARGS               = 0x80
ACC_NATIVE                = 0x100
ACC_ABSTRACT              = 0x400
ACC_STRICT                = 0x800
ACC_SYNTHETIC             = 0x1000
