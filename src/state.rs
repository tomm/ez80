use super::registers::*;

/// ez80 opcode "suffixes". we call them prefixes here
/// because they appear before the opcode in machine code
#[derive(Clone,Copy,Debug)]
pub enum SizePrefix {
    None,
    LIL,
    LIS,
    SIL,
    SIS
}

/// Internal state of the CPU
/// 
/// Stores the state of the registers and additional hidden execution
/// state of the CPU.
pub struct State {
    /// Values of the Z80 registers
    pub reg: Registers,
    /// Halt state of the CPU
    pub halted: bool,
    /// Non maskable interrupt signaled
    pub nmi_pending: bool,
    /// Reset signaled
    pub reset_pending: bool,
    // Alternate index management
    pub index: Reg16, // Using HL, IX or IY
    pub displacement: i8, // Used for (IX+d) and (iY+d)
    pub sz_prefix: SizePrefix,
    pub instructions_executed: u64,
}

impl State {
    /// Returns the initial state of a Z80 on power up
    pub fn new() -> State {
        State {
            reg: Registers::new(),
            halted: false,
            nmi_pending: false,
            reset_pending: false,
            index: Reg16::HL,
            displacement: 0,
            sz_prefix: SizePrefix::None,
            instructions_executed: 0,
        }
    }

    pub fn is_op_long(&self) -> bool {
        match self.sz_prefix {
            SizePrefix::None => self.reg.adl,
            SizePrefix::LIL => true,
            SizePrefix::LIS => true,
            SizePrefix::SIL => false,
            SizePrefix::SIS => false
        }
    }

    pub fn is_imm_long(&self) -> bool {
        match self.sz_prefix {
            SizePrefix::None => self.reg.adl,
            SizePrefix::LIL => true,
            SizePrefix::LIS => false,
            SizePrefix::SIL => true,
            SizePrefix::SIS => false
        }
    }

    pub fn sp(&self) -> u32 {
        if self.is_op_long() {
            self.reg.get24(Reg16::SP)
        } else {
            self.reg.get16_mbase(Reg16::SP)
        }
    }

    /// Returns the program counter
    #[inline]
    pub fn pc(&self) -> u32 {
        if self.reg.adl {
            self.reg.pc
        } else {
            ((self.reg.mbase as u32) << 16) + (self.reg.pc & 0xffff) as u32
        }
    }

    pub fn set_pc(&mut self, value: u32) {
        self.reg.pc = value & 0xffffff;
    }
}

impl std::fmt::Display for SizePrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", match self {
            &SizePrefix::LIL => ".LIL",
            &SizePrefix::LIS => ".LIS",
            &SizePrefix::SIL => ".SIL",
            &SizePrefix::SIS => ".SIS",
            &SizePrefix::None => "",

        })
    }
}
