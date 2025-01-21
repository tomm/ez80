use super::opcode::*;
use super::environment::*;
use super::registers::*;
use super::state::SizePrefix;

// Relative jumps
pub fn build_djnz() -> Opcode {
    Opcode {
        name: "DJNZ l".to_string(),
        action: Box::new(move |env: &mut Environment| {
            let offset = env.advance_pc();
            let b = env.state.reg.get8(Reg8::B).wrapping_add(0xff /* -1 */);
            env.state.reg.set8(Reg8::B, b);
            if b != 0 {
                // Condition not met
                env.sys.use_cycles(1);
                relative_jump(env, offset);
            }
        })
    }
}

pub fn build_jr_unconditional() -> Opcode {
    Opcode {
        name: "JR l".to_string(),
        action: Box::new(move |env: &mut Environment| {
            let offset = env.advance_pc();
            env.sys.use_cycles(1);
            relative_jump(env, offset);
        })
    }
}

pub fn build_jr_eq((flag, value, name): (Flag, bool, &str)) -> Opcode {
    Opcode {
        name: format!("JR {}, l", name),
        action: Box::new(move |env: &mut Environment| {
            let offset = env.advance_pc();
            if env.state.reg.get_flag(flag) == value {
                env.sys.use_cycles(2);
                relative_jump(env, offset);
            }
        })
    }
}


fn relative_jump(env: &mut Environment, offset: u8) {
    let mut pc = env.state.pc();
    pc = env.wrap_address(pc, offset as i8 as i32);
    env.state.set_pc(pc);
}

fn handle_jump_adl_state(env: &mut Environment) {
    if env.state.reg.adl {
        match env.state.sz_prefix {
            SizePrefix::SIS => { env.state.reg.adl = false },
            SizePrefix::LIS | SizePrefix::SIL => {
                eprintln!("Invalid size prefix for ADL=1 with jump at PC=${:x}", env.state.pc());
            }
            SizePrefix::LIL |
            SizePrefix::None => {}
        }
    } else {
        match env.state.sz_prefix {
            SizePrefix::LIL => { env.state.reg.adl = true },
            SizePrefix::LIS | SizePrefix::SIL => {
                eprintln!("Invalid size prefix for ADL=0 with jump at PC=${:x}", env.state.pc());
            },
            SizePrefix::SIS | SizePrefix::None => {}
        }
    }
}

// Absolute jumps
pub fn build_jp_unconditional() -> Opcode {
    Opcode {
        name: "JP nn".to_string(),
        action: Box::new(move |env: &mut Environment| {
            let address = env.advance_immediate_16mbase_or_24();
            handle_jump_adl_state(env);
            env.sys.use_cycles(1);
            env.state.set_pc(address);
        })
    }
}

pub fn build_jp_eq((flag, value, name): (Flag, bool, &str)) -> Opcode {
    Opcode {
        name: format!("JP {}, nn", name),
        action: Box::new(move |env: &mut Environment| {
            let address = env.advance_immediate_16mbase_or_24();
            if env.state.reg.get_flag(flag) == value {
                env.sys.use_cycles(1);
                env.state.set_pc(address);
            }
        })
    }
}

pub fn build_jp_hl() -> Opcode {
    Opcode {
        name: "JP (HL)".to_string(),
        action: Box::new(move |env: &mut Environment| {
            // Note: no displacement added to the index
            let address = env.index_value();
            env.sys.use_cycles(1);
            env.state.set_pc(address);
        })
    }
}

fn handle_call_size_prefix(env: &mut Environment) {
    let pc = env.state.pc();

    if env.state.reg.adl {
        match env.state.sz_prefix {
            SizePrefix::None => {
                env.push(pc); // 3 bytes onto SPL
            },
            SizePrefix::SIS | // not valid according to docs, but works
            SizePrefix::LIS => {
                env.push_byte_sps((pc >> 8) as u8);
                env.push_byte_sps(pc as u8);
                env.push_byte_spl((pc >> 16) as u8);
                env.push_byte_spl(3);
                env.state.reg.adl = false;
            }
            SizePrefix::LIL => {
                env.push(pc); // 3 bytes onto SPL
                env.push_byte_spl(3);
            }
            prefix => {
                env.push(pc); // 3 bytes onto SPL
                eprintln!("invalid call size prefix for ADL=1: {}", prefix);
            }
        }
    } else {
        match env.state.sz_prefix {
            SizePrefix::None => {
                env.push_byte_sps((pc >> 8) as u8);
                env.push_byte_sps(pc as u8);
            },
            SizePrefix::LIL | // not valid according to docs, but works
            SizePrefix::SIL => {
                env.push_byte_spl((pc >> 8) as u8);
                env.push_byte_spl(pc as u8);
                env.push_byte_spl(2);
                env.state.reg.adl = true;
            }
            SizePrefix::SIS => {
                env.push_byte_sps((pc >> 8) as u8);
                env.push_byte_sps(pc as u8);
                env.push_byte_spl(2);
            }
            SizePrefix::LIS => {
                env.push_byte_spl((pc >> 8) as u8);
                env.push_byte_spl(pc as u8);
                eprintln!("invalid call size prefix for ADL=0: LIS");
            }
        }
    }
}

// Calls to subroutine
pub fn build_call() -> Opcode {
    Opcode {
        name: "CALL nn".to_string(),
        action: Box::new(move |env: &mut Environment| {
            let address = env.advance_immediate16or24();
            handle_call_size_prefix(env);
            env.state.set_pc(address);
        })
    }
}

pub fn build_call_eq((flag, value, name): (Flag, bool, &str)) -> Opcode {
    Opcode {
        name: format!("CALL {}, nn", name),
        action: Box::new(move |env: &mut Environment| {
            let address = env.advance_immediate_16mbase_or_24();
            if env.state.reg.get_flag(flag) == value {
                handle_call_size_prefix(env);
                env.state.set_pc(address);
            }
        })
    }
}

fn handle_rst_size_prefix(env: &mut Environment, vec: u32) {
    let pc = env.state.pc();

    if env.state.reg.adl {
        match env.state.sz_prefix {
            SizePrefix::None => {
                env.push(pc);
                env.state.set_pc(vec);
            },
            SizePrefix::SIL => {
                env.push_byte_sps((pc >> 8) as u8);
                env.push_byte_sps(pc as u8);
                env.push_byte_spl((pc >> 16) as u8);
                env.push_byte_spl(3);
                env.state.reg.pc = vec;
                env.state.reg.adl = false;
            }
            SizePrefix::LIS | // not valid according to spec, but works on ez80
            SizePrefix::LIL => {
                env.push(pc);
                env.push_byte_spl(3);
                env.state.reg.pc = vec;
            }
            SizePrefix::SIS => {
                eprintln!("invalid rst size prefix");
            }
        }
    } else {
        match env.state.sz_prefix {
            SizePrefix::None => {
                env.push(pc);
                env.state.set_pc(vec);
            },
            SizePrefix::SIL | // <- SIL forbidden by spec, but works on ez80
            SizePrefix::SIS => {
                env.push(pc);
                env.push_byte_spl(2);
                env.state.reg.pc = vec;
            }
            SizePrefix::LIL | // <- LIL is forbidden by the spec, but work on ez80
            SizePrefix::LIS => {
                env.push_byte_spl((pc >> 8) as u8);
                env.push_byte_spl(pc as u8);
                env.push_byte_spl(2);
                env.state.reg.adl = true;
                env.state.reg.pc = vec;
            }
        }
    }
}
pub fn build_rst(d: u8) -> Opcode {
    Opcode {
        name: format!("RST {:02x}h", d),
        action: Box::new(move |env: &mut Environment| {
            let address = d as u32;
            handle_rst_size_prefix(env, address);
        })
    }
}

// Returns

pub fn build_ret() -> Opcode {
    Opcode {
        name: "RET".to_string(),
        action: Box::new(move |env: &mut Environment| {
            env.sys.use_cycles(2);
            env.subroutine_return();
        })
    }
}

pub fn build_reti() -> Opcode {
    Opcode {
        name: "RETI".to_string(),
        action: Box::new(move |env: &mut Environment| {
            env.subroutine_return();
        })
    }
}

pub fn build_retn() -> Opcode {
    Opcode {
        name: "RETN".to_string(),
        action: Box::new(move |env: &mut Environment| {
            env.subroutine_return();
            env.state.reg.end_nmi();
        })
    }
}

pub fn build_ret_eq((flag, value, name): (Flag, bool, &str)) -> Opcode {
    Opcode {
        name: format!("RET {}", name),
        action: Box::new(move |env: &mut Environment| {
            if env.state.reg.get_flag(flag) == value {
                env.subroutine_return();
            }
        })
    }
}
