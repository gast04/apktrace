/*
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
*/

#[allow(unused_variables)]
// not sure how this allow works...

pub const VERSION_SIG: (u8, u8)               = (1, 1);
pub const ALLCLASSES_SIG: (u8, u8)            = (1, 3);
pub const IDSIZES_SIG: (u8, u8)               = (1, 7);
pub const SUSPENDVM_SIG: (u8, u8)             = (1, 8);
pub const RESUMEVM_SIG:  (u8, u8)             = (1, 9);

pub const METHODS_SIG:  (u8, u8)              = (2, 5);


pub const THREADNAME_SIG: (u8, u8)            = (11, 1);

pub const EVENTSET_SIG: (u8, u8)              = (15, 1);


// ERROR codes
pub const ILLEGAL_ARGUMENT: u8                = 103;


// Event request Codes
pub const EVENT_SINGLE_STEP: u8               = 1;
pub const EVENT_BREAKPOINT: u8                = 2;
pub const EVENT_CLASS_PREPARE: u8             = 8;
pub const EVENT_CLASS_LOAD: u8                = 10;
pub const EVENT_METHOD_ENTRY: u8              = 40;
pub const EVENT_METHOD_EXIT: u8               = 41;
pub const EVENT_METHOD_EXIT_WRV: u8           = 42;   // WRV = WITH_RETURN_VALUE


// mod kinds
pub const MODKIND_CLASS_MATCH: u8             = 5;
pub const MODKIND_CLASS_EXCLUDE: u8           = 6;


// other codes
pub const SUSPEND_ALL:u8                      = 2;




