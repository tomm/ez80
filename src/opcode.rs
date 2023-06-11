use super::state::SizePrefix;
use super::environment::*;
use super::registers::*;

type OpcodeFn = dyn Fn(&mut Environment);

pub struct Opcode {
    pub name: String,
    pub action: Box<OpcodeFn>,
}

impl Opcode {
    pub fn execute(&self, env: &mut Environment) {
        (self.action)(env);
    }

    /// returns String, and u32 PC increment due to immediates
    /// (the PC increment due to the opcode itself, (and due 
    /// to the state.index hack), have already been applied by
    /// the decoder.
    pub fn disasm(&self, env: &Environment) -> (String, u32) {
        let mut name = if self.name.contains("__index") {
            self.name.replace("__index", &env.index_description())
        } else {
            self.name.clone()
        };

        match env.state.sz_prefix {
            SizePrefix::None => {}
            _ => {
                if let Some(after_opcode_pos) = name.find(' ') {
                    name.insert_str(after_opcode_pos, &env.state.sz_prefix.to_string());
                }
            }
        }

        // hack. when the env.state.index trick is in use to change HL
        // to IX or IY, modify the opcode text. Note that for opcodes
        // where HL can appear as the non-index operand (eg LD HL,(IX+1)),
        // this case is not triggered, because env.state.index is not
        // used (see prefix_dd in decoder_ez80).
        match env.state.index {
            Reg16::IX => { name = name.replace("HL", "IX"); }
            Reg16::IY => { name = name.replace("HL", "IY"); }
            _ => {}
        }

        if name.contains("nn") {
            if env.state.is_imm_long() {
                // Immediate argument 24 bits
                let nn = env.peek24_pc();
                let nn_str = format!("${:x}", nn);
                (name.replace("nn", &nn_str), 3)
            } else {
                // Immediate argument 16 bits
                let nn = env.peek16_pc();
                let nn_str = format!("${:x}", nn);
                (name.replace("nn", &nn_str), 2)
            }
        } else if name.contains('n') {
            // Immediate argument 8 bits
            let n = env.peek_pc();
            let n_str = format!("${:x}", n);
            (name.replace('n', &n_str), 1)
        } else if name.contains('d') {
            // Immediate argument 8 bits signed
            // In assembly it's shown with 2 added as if it were from the opcode pc.
            let d = env.peek_pc() as i8 as i16;
            let d_str = if d < 0 { format!("-${:x}", -d) } else { format!("+${:x}", d) };
            (name.replace('d', &d_str), 1)
        } else if name.contains('l') {
            // Jump offset as 8 bits signed.
            // In asm show as absolute address
            let addr = (env.state.pc() as i32 + 1 + env.peek_pc() as i8 as i32) as u32;
            let l_str = format!("${:x}", addr);
            (name.replace('l', &l_str), 1)
        } else {
            (name, 0)
        }
    }
}

pub fn build_nop() -> Opcode {
    Opcode {
        name: "NOP".to_string(),
        action: Box::new(|_: &mut Environment| {
            // Nothing done
        })
    }
}

pub fn build_noni_nop() -> Opcode {
    Opcode {
        name: "NONINOP".to_string(),
        action: Box::new(|_: &mut Environment| {
            // Nothing done
        })
    }
}

pub fn build_halt() -> Opcode {
    Opcode {
        name: "HALT".to_string(),
        action: Box::new(move |env: &mut Environment| {
            env.state.halted = true;
        })
    }
}

pub fn build_pop_rr(rr: Reg16) -> Opcode {
    Opcode {
        name: format!("POP {:?}", rr),
        action: Box::new(move |env: &mut Environment| {
            let value = env.pop();
            if env.state.is_op_long() && rr != Reg16::AF {
                env.set_reg24(rr, value);
            } else {
                env.set_reg16(rr, value as u16);
            }
        })
    }
}

pub fn build_push_rr(rr: Reg16) -> Opcode {
    Opcode {
        name: format!("PUSH {:?}", rr),
        action: Box::new(move |env: &mut Environment| {
            let value = env.reg16or24_ext(rr);
            env.push(value);
        })
    }
}

pub fn build_conf_interrupts(enable: bool) -> Opcode {
    let name = if enable {"EI"} else  {"DI"};
    Opcode {
        name: name.to_string(),
        action: Box::new(move |env: &mut Environment| {
            env.state.reg.set_interrupts(enable);
        })
    }
}

pub fn build_im(im: u8) -> Opcode {
    Opcode {
        name: format!("IM {}", im),
        action: Box::new(move |env: &mut Environment| {
            env.state.reg.set_interrupt_mode(im);
        })
    }
}

pub fn build_stmix() -> Opcode {
    Opcode {
        name: "STMIX".to_string(),
        action: Box::new(move |env: &mut Environment| {
            env.state.reg.madl = true;
        })
    }
}

pub fn build_rsmix() -> Opcode {
    Opcode {
        name: "RSMIX".to_string(),
        action: Box::new(move |env: &mut Environment| {
            env.state.reg.madl = false;
        })
    }
}
