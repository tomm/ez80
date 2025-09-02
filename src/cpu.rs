use super::decoder_ez80::*;
use super::decoder_z80::*;
use super::decoder_8080::*;
use super::environment::*;
use super::machine::*;
use super::opcode::*;
use super::registers::*;
use super::state::*;

const NMI_ADDRESS: u32 = 0x0066;

/// The Z80 cpu emulator.
/// 
/// Executes Z80 instructions changing the cpu State and Machine
pub struct Cpu {
    pub state: State,
    trace: bool,
    decoder: Box<dyn Decoder>,
}

pub(crate) trait Decoder {
    fn decode(&self, env: &mut Environment) -> &Opcode;
}

impl Cpu {

    /// Returns a Z80 Cpu instance. Alias of new_z80()
    pub fn new() -> Cpu {
        Self::new_z80()
    }

    /// Returns a Z80 Cpu instance
    pub fn new_z80() -> Cpu {
        Cpu {
            state: State::new(),
            trace: false,
            decoder: Box::new(DecoderZ80::new())
        }
    }

    pub fn new_ez80() -> Cpu {
        Cpu {
            state: State::new(),
            trace: false,
            decoder: Box::new(DecoderEZ80::new())
        }
    }

    /// Returns an Intel 8080 Cpu instance
    pub fn new_8080() -> Cpu {
        let mut cpu = Cpu {
            state: State::new(),
            trace: false,
            decoder: Box::new(Decoder8080::new())
        };

        cpu.state.reg.set_8080();
        cpu
    }

}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    /// Executes a single instruction
    ///
    /// # Arguments
    ///
    /// * `sys` - A representation of the emulated machine that has the Machine trait
    ///
    pub fn execute_instruction(&mut self, sys: &mut dyn Machine) {
        if self.is_halted() {
            // The CPU is in HALT state. Only interrupts can execute.
            return
        }

        let mut env = Environment::new(&mut self.state, sys);
        if env.state.reset_pending {
            env.state.reset_pending = false;
            env.state.nmi_pending = false;
            env.state.halted = false;
            env.state.set_pc(0x0000);
            env.state.reg.set8(Reg8::I, 0x00);
            env.state.reg.set8(Reg8::R, 0x00);
            env.state.reg.set_interrupts(false);
            env.state.reg.set_interrupt_mode(0);
        }
        else if env.state.nmi_pending {
            env.state.nmi_pending = false;
            env.state.halted = false;
            env.state.reg.start_nmi();
            env.subroutine_call(NMI_ADDRESS);
        }

        let pc = env.state.pc();
        let opcode = self.decoder.decode(&mut env);
        if self.trace {
            print!("==> {:06x}: {:20}", pc, opcode.disasm(&env).0);
        }
        opcode.execute(&mut env);
        env.clear_index();
        env.state.clear_sz_prefix();
        env.state.instructions_executed += 1;
        env.state.reg.set8(Reg8::R, env.state.reg.get8(Reg8::R).wrapping_add(1));

        if self.trace {
            print!(" PC:{:06x} AF:{:04x} BC:{:06x} DE:{:06x} HL:{:06x} SPS:{:04x} SPL:{:06x} IX:{:06x} IY:{:06x} MB {:02x} ADL {:01x} MADL {:01x} tick {}",
                self.state.pc(),
                self.state.reg.get16(Reg16::AF),
                self.state.reg.get24(Reg16::BC),
                self.state.reg.get24(Reg16::DE),
                self.state.reg.get24(Reg16::HL),
                self.state.reg.get16(Reg16::SP),
                self.state.reg.get24(Reg16::SP),
                self.state.reg.get24(Reg16::IX),
                self.state.reg.get24(Reg16::IY),
                self.state.reg.mbase,
                self.state.reg.adl as i32,
                self.state.reg.madl as i32,
                self.state.instructions_executed,
            );
            println!(" [{:02x} {:02x} {:02x} {:02x}]", sys.peek(pc),
                sys.peek(pc.wrapping_add(1)),
                sys.peek(pc.wrapping_add(2)),
                sys.peek(pc.wrapping_add(3)));
        }
    }

    /// Returns the instrction in PC disassembled. PC is advanced.
    /// 
    /// # Arguments
    /// 
    /// * `sys` - A representation of the emulated machine that has the Machine trait
    ///  
    pub fn disasm_instruction(&mut self, sys: &mut dyn Machine) -> String {
        let mut env = Environment::new(&mut self.state, sys);
        let opcode = self.decoder.decode(&mut env);
        let (asm, pc_inc) = opcode.disasm(&env);
        for _ in 0..pc_inc { env.advance_pc(); }
        asm
    }

    /// Activates or deactivates traces of the instruction executed and
    /// the state of the registers.
    /// 
    /// # Arguments
    /// 
    /// * `trace` - A bool defining the trace state to set
    pub fn set_trace(&mut self, trace: bool) {
        self.trace = trace;
    }

    /// Set eZ80 ADL state
    pub fn set_adl(&mut self, adl: bool) {
        self.state.reg.adl = adl;
    }

    /// Returns a Registers struct to read and write on the Z80 registers
    pub fn registers(&mut self) -> &mut Registers {
        &mut self.state.reg
    }

    /// Returns if the Cpu has executed a HALT
    pub fn is_halted(&self) -> bool {
        self.state.halted && !self.state.nmi_pending && !self.state.reset_pending
    }

    /// Non maskable interrupt request
    pub fn signal_nmi(&mut self) {
        self.state.nmi_pending = true
    }

    /// Signal reset
    pub fn signal_reset(&mut self) {
        self.state.reset_pending = true
    }
}


