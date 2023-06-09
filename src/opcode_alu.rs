use super::opcode::*;
use super::environment::*;
use super::registers::*;
use super::operators::*;

pub fn build_lea_rr_ind_offset(dest: Reg16, src: Reg16) -> Opcode {
    Opcode {
        name: format!("LEA {:?}, {:?}d", dest, src),
        action: Box::new(move |env: &mut Environment| {
            let imm = env.advance_pc() as i8 as i32 as u32;
            if env.state.is_op_long() {
                let value = env.state.reg.get24(src).wrapping_add(imm);
                env.state.reg.set24(dest, value);
            } else {
                let value = env.state.reg.get16(src).wrapping_add(imm as u16);
                env.state.reg.set16(dest, value);
            }
        })
    }
}

pub fn build_pea(src: Reg16) -> Opcode {
    Opcode {
        name: format!("PEA {:?}d", src),
        action: Box::new(move |env: &mut Environment| {
            let imm = env.advance_pc() as i8 as i32 as u32;
            if env.state.is_op_long() {
                let value = env.state.reg.get24(src).wrapping_add(imm);
                env.push(value);
            } else {
                let value = env.state.reg.get16(src).wrapping_add(imm as u16);
                env.push(value as u32);
            }
        })
    }
}

pub fn build_tst_a_r(reg: Reg8) -> Opcode {
    Opcode {
        name: format!("TST A, {}", reg),
        action: Box::new(move |env: &mut Environment| {
            let a = env.state.reg.a();
            let b = env.reg8_ext(reg);
            operator_tst(env, a, b);
        })
    }
}

pub fn build_tst_a_n() -> Opcode {
    Opcode {
        name: format!("TST A, n"),
        action: Box::new(move |env: &mut Environment| {
            let a = env.state.reg.a();
            let b = env.advance_pc();
            operator_tst(env, a, b);
        })
    }
}

pub fn build_operator_a_idx_offset(idx: Reg16, (op, name): (Operator, &str)) -> Opcode {
    Opcode {
        name: format!("{} A, ({:?}d)", name, idx),
        action: Box::new(move |env: &mut Environment| {
            let offset = env.advance_pc() as i8 as i32 as u32;
            let a = env.state.reg.a();
            let address = if env.state.is_op_long() {
                env.state.reg.get24(idx).wrapping_add(offset)
            } else {
                env.state.reg.get16_mbase_offset(idx, offset as u16)
            };
            let b = env.peek(address);
            let v = op(env, a, b);
            env.state.reg.set_a(v);
        })
    }
}

pub fn build_operator_a_r(r: Reg8, (op, name): (Operator, &str)) -> Opcode {
    if r != Reg8::_HL && r != Reg8::H && r != Reg8::L {
        // Fast version
        Opcode {
            name: format!("{} A, {}", name, r),
            action: Box::new(move |env: &mut Environment| {
                let a = env.state.reg.a();
                let b = env.state.reg.get8(r);
                let v = op(env, a, b);
                env.state.reg.set_a(v);
            })
        }
    } else {
        Opcode {
            name: format!("{} A, {}", name, r),
            action: Box::new(move |env: &mut Environment| {
                let a = env.state.reg.a();
                let b = env.reg8_ext(r);
                let v = op(env, a, b);

                env.state.reg.set_a(v);
            })
        }
    }
}

pub fn build_operator_a_n((op, name): (Operator, &str)) -> Opcode {
    Opcode {
        name: format!("{} A, n", name),
        action: Box::new(move |env: &mut Environment| {
            let a = env.state.reg.a();
            let b = env.advance_pc();
            let v = op(env, a, b);

            env.state.reg.set_a(v);
        })
    }
}

pub fn build_cp_block((inc, repeat, postfix) : (bool, bool, &'static str)) -> Opcode {
    Opcode {
        name: format!("CP{}", postfix),
        action: Box::new(move |env: &mut Environment| {
            let a = env.state.reg.a();
            let b = env.reg8_ext(Reg8::_HL);
            let c_bak = env.state.reg.get_flag(Flag::C);
            operator_cp(env, a, b);
            let bc = if env.state.is_op_long() {
                env.state.reg.inc_dec24(Reg16::HL, inc);
                env.state.reg.inc_dec24(Reg16::BC, false /*decrement*/)
            } else {
                env.state.reg.inc_dec16(Reg16::HL, inc);
                env.state.reg.inc_dec16(Reg16::BC, false /*decrement*/)
            };

            // TUZD-4.2
            let mut n = a.wrapping_sub(b);
            if env.state.reg.get_flag(Flag::H) {
                n = n.wrapping_sub(1);
            }
            env.state.reg.update_undocumented_flags_block(n);
            env.state.reg.set_flag(Flag::N);
            env.state.reg.put_flag(Flag::P, bc != 0);
            env.state.reg.put_flag(Flag::C, c_bak); // C unchanged
            // S, Z and H set by operator_cp()

            if repeat && bc != 0 &&  a != b {
                // Back to redo the instruction
                let pc = env.wrap_address(env.state.pc(), -2);
                env.state.set_pc(pc);
            }
        })
    }
}

pub fn build_mlt_rr(reg: Reg16) -> Opcode {
    Opcode {
        name: format!("MLT {:?}", reg),
        action: Box::new(move |env: &mut Environment| {
            let r = env.state.reg.get16(reg);
            let a = r & 0xff;
            let b = (r >> 8) & 0xff;
            env.state.reg.set16(reg, a * b);
        })
    }
}
