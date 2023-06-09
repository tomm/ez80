use crate::machine::Machine;
use crate::cpu::Cpu;
use crate::environment::Environment;
use crate::registers::*;

#[derive(Clone, Debug)]
pub struct Disasm {
    pub loc: u32,
    pub asm: String,
    pub bytes: Vec<u8>
}

/**
 * Disassemble a section of code.
 *
 * Tries to not mutate state, but needs a mutable cpu ref...
 * iz80 disassembly is a bit awkward due to the way it increments the PC
 */
pub fn disassemble(machine: &mut dyn Machine, cpu: &mut Cpu, adl_override: Option<bool>, start: u32, end: u32) -> Vec<Disasm> {
    let mut dis: Vec<Disasm> = vec![];
    let old_state = cpu.state.clone();

    if let Some(adl) = adl_override {
        cpu.state.reg.adl = adl;
    }
    cpu.state.reg.pc = start;
    cpu.state.reg.mbase = (start >> 16) as u8;

    while cpu.state.pc() < end {

        let opcode_start = cpu.state.pc();
        let (opcode_asm, opcode) = cpu.disasm_instruction(machine);

        // mega hack. disasm_instruction advances the PC, but 
        // not over immediate values. the only way we have of
        // determining if there was an intermediate is by the
        // presence of 'nn' formatting sequence in opcode.name
        if opcode.name.contains("nn") {
            if cpu.state.is_imm_long() {
                cpu.state.set_pc(cpu.state.pc() + 3);
            } else {
                cpu.state.set_pc(cpu.state.pc() + 2);
            }
        }
        else if opcode.name.contains("n") || opcode.name.contains("d") {
            cpu.state.set_pc(cpu.state.pc() + 1);
        }

        // horrible. but adl/non adl wraparound is a pain
        let mut instruction_bytes = vec![];
        {
            let opcode_end = cpu.state.pc();
            let mut env = Environment::new(&mut cpu.state, machine);
            env.state.reg.pc = opcode_start;
            while env.state.reg.pc != opcode_end {
                instruction_bytes.push(env.advance_pc());
            }
        }

        dis.push(Disasm {
            loc: opcode_start,
            asm: opcode_asm,
            bytes: instruction_bytes
        });

        cpu.state.clear_sz_prefix();
        cpu.state.index = Reg16::HL;

        // handle pc wraparound in ADL=0 mode
        if cpu.state.reg.pc < opcode_start {
            cpu.state.reg.mbase += 1;
        }
    }

    // restore old cpu state
    cpu.state = old_state;

    dis
}
