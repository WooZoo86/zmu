//!
//! A Trait for representing a Cortex armv6-m exceptions.
//!
//!

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Exception {
    Reset,
    NMI,
    HardFault,
    MemoryManagementFault,
    BusFault,
    UsageFault,
    Reserved4,
    Reserved5,
    Reserved6,
    DebugMonitor,
    SVCall,
    Reserved8,
    Reserved9,
    PendSV,
    SysTick,
    Interrupt { n: u8 },
}

impl From<Exception> for u8 {
    fn from(value: Exception) -> Self {
        match value {
            Exception::Reset => 1,
            Exception::NMI => 2,
            Exception::HardFault => 3,
            Exception::MemoryManagementFault => 4,
            Exception::BusFault => 5,
            Exception::UsageFault => 6,
            Exception::Reserved4 => 7,
            Exception::Reserved5 => 8,
            Exception::Reserved6 => 9,
            Exception::DebugMonitor => 10,
            Exception::SVCall => 11,
            Exception::Reserved8 => 12,
            Exception::Reserved9 => 13,
            Exception::PendSV => 14,
            Exception::SysTick => 15,
            Exception::Interrupt { n } => 16 + n,
        }
    }
}

impl From<u8> for Exception {
    fn from(value: u8) -> Self {
        match value {
            1 => Exception::Reset,
            2 => Exception::NMI,
            3 => Exception::HardFault,
            4 => Exception::MemoryManagementFault,
            5 => Exception::BusFault,
            6 => Exception::UsageFault,
            7 => Exception::Reserved4,
            8 => Exception::Reserved5,
            9 => Exception::Reserved6,
            10 => Exception::DebugMonitor,
            11 => Exception::SVCall,
            12 => Exception::Reserved8,
            13 => Exception::Reserved9,
            14 => Exception::PendSV,
            15 => Exception::SysTick,
            _ => Exception::Interrupt { n: value - 16 },
        }
    }
}
