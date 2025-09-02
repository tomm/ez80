use super::cpu::*;
use super::opcode::*;
use super::opcode_alu::*;
use super::opcode_arith::*;
use super::opcode_io::*;
use super::opcode_bits::*;
use super::opcode_jumps::*;
use super::opcode_ld::*;
use super::operators::*;
use super::registers::*;
use super::environment::*;
use super::state::*;

/* See
    http://www.z80.info/decoding.htm
    http://clrhome.org/table/
    http://z80-heaven.wikidot.com/instructions-set
*/

pub struct DecoderEZ80 {
    no_prefix: [Option<Opcode>; 256],
    prefix_cb: [Option<Opcode>; 256],
    prefix_cb_indexed: [Option<Opcode>; 256],
    prefix_ed: [Option<Opcode>; 256],
    // prefix_dd & prefix_fd are only used for a few ez80 instructions.
    // the rest of those prefixes are handled by the environment.index hack
    prefix_dd: [Option<Opcode>; 256],
    prefix_fd: [Option<Opcode>; 256],
    has_displacement: [bool; 256],
}

impl Decoder for DecoderEZ80 {
    fn decode(&self, env: &mut Environment) -> &Opcode {
        let mut b0 = env.advance_pc();

        // Process prefixes even if reapeated
        loop {
            match b0 {
                0x40 => env.state.sz_prefix = SizePrefix::SIS,
                0x49 => env.state.sz_prefix = SizePrefix::LIS,
                0x52 => env.state.sz_prefix = SizePrefix::SIL,
                0x5B => env.state.sz_prefix = SizePrefix::LIL,
                _ => break,
            }
            b0 = env.advance_pc();
        }
        loop {
            match b0 {
                0xdd => env.set_index(Reg16::IX),
                0xfd => env.set_index(Reg16::IY),
                _ => break,
            }
            b0 = env.advance_pc();
        }
        
        let opcode = match b0 {
            0xcb => {
                if env.is_alt_index() {
                    env.load_displacement();
                    &self.prefix_cb_indexed[env.advance_pc() as usize]
                } else {
                    &self.prefix_cb[env.advance_pc() as usize]
                }
            },
            0xed => {
                env.clear_index(); // With ed, the current prefix is ignored
                &self.prefix_ed[env.advance_pc() as usize]
            },
            // XXX hack. should put all dd, fd opcodes in this table
            0x0f | 0x1f | 0x2f | 0x07 | 0x17 | 0x27 | 0x31 | 0x37 | 0x3e | 0x3f | 0x86
                | 0x96 | 0xa6 | 0xb6 | 0x8e | 0x9e | 0xae | 0xbe if env.is_alt_index() => {
                match env.get_index() {
                    Reg16::IX => {
                        env.clear_index();
                        &self.prefix_dd[b0 as usize]
                    }
                    Reg16::IY => {
                        env.clear_index();
                        &self.prefix_fd[b0 as usize]
                    }
                    _ => panic!("bug")
                }
            },
            _ => {
                if self.has_displacement[b0 as usize] && env.is_alt_index() {
                    env.load_displacement();
                }
                &self.no_prefix[b0 as usize]
            }
        };
        match opcode {
            Some(o) => o,
            None => {
                panic!("Opcode {:02x} not defined", b0);
            }
        }
    }
}

impl DecoderEZ80 {
    pub fn new() -> DecoderEZ80 {

        let mut decoder = DecoderEZ80 {
            no_prefix: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            prefix_cb: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            prefix_cb_indexed: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            prefix_dd: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            prefix_ed: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            prefix_fd: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            has_displacement: [false; 256],
        };
        decoder.load_no_prefix();
        decoder.load_prefix_cb();
        decoder.load_prefix_cb_indexed();
        decoder.load_prefix_ed();
        decoder.load_prefix_dd();
        decoder.load_prefix_fd();
        decoder.load_has_displacement();
        decoder
    }

    /* Only some ez80 instructions are implemented here. Most use state.index hack */
    fn load_prefix_dd(&mut self) {
        for c in 0..=255 {
            let opcode = match c {
                0x07 => Some(build_ld_rr_idx_disp(Reg16::BC, Reg16::IX)),
                0x0f => Some(build_ld_idx_disp_rr(Reg16::IX, Reg16::BC)),
                0x17 => Some(build_ld_rr_idx_disp(Reg16::DE, Reg16::IX)),
                0x1f => Some(build_ld_idx_disp_rr(Reg16::IX, Reg16::DE)),
                0x27 => Some(build_ld_rr_idx_disp(Reg16::HL, Reg16::IX)),
                0x2f => Some(build_ld_idx_disp_rr(Reg16::IX, Reg16::HL)),
                0x31 => Some(build_ld_rr_idx_disp(Reg16::IY, Reg16::IX)),
                0x37 => Some(build_ld_rr_idx_disp(Reg16::IX, Reg16::IX)),
                0x3e => Some(build_ld_idx_disp_rr(Reg16::IX, Reg16::IY)),
                0x3f => Some(build_ld_idx_disp_rr(Reg16::IX, Reg16::IX)),
                0x86 => Some(build_operator_a_idx_offset(Reg16::IX, (operator_add, "ADD"))),
                0x8e => Some(build_operator_a_idx_offset(Reg16::IX, (operator_adc, "ADC"))),
                0x96 => Some(build_operator_a_idx_offset(Reg16::IX, (operator_sub, "SUB"))),
                0x9e => Some(build_operator_a_idx_offset(Reg16::IX, (operator_sbc, "SBC"))),
                0xa6 => Some(build_operator_a_idx_offset(Reg16::IX, (operator_and, "AND"))),
                0xae => Some(build_operator_a_idx_offset(Reg16::IX, (operator_xor, "XOR"))),
                0xb6 => Some(build_operator_a_idx_offset(Reg16::IX, (operator_or, "OR"))),
                0xbe => Some(build_operator_a_idx_offset(Reg16::IX, (operator_cp, "CP"))),

                _ => None
            };
            self.prefix_dd[c as usize] = opcode;
        }
    }

    /* Only some ez80 instructions are implemented here. Most use state.index hack */
    fn load_prefix_fd(&mut self) {
        for c in 0..=255 {
            let opcode = match c {
                0x07 => Some(build_ld_rr_idx_disp(Reg16::BC, Reg16::IY)),
                0x0f => Some(build_ld_idx_disp_rr(Reg16::IY, Reg16::BC)),
                0x17 => Some(build_ld_rr_idx_disp(Reg16::DE, Reg16::IY)),
                0x1f => Some(build_ld_idx_disp_rr(Reg16::IY, Reg16::DE)),
                0x27 => Some(build_ld_rr_idx_disp(Reg16::HL, Reg16::IY)),
                0x2f => Some(build_ld_idx_disp_rr(Reg16::IY, Reg16::HL)),
                0x31 => Some(build_ld_rr_idx_disp(Reg16::IX, Reg16::IY)),
                0x37 => Some(build_ld_rr_idx_disp(Reg16::IY, Reg16::IY)),
                0x3e => Some(build_ld_idx_disp_rr(Reg16::IY, Reg16::IX)),
                0x3f => Some(build_ld_idx_disp_rr(Reg16::IY, Reg16::IY)),
                0x86 => Some(build_operator_a_idx_offset(Reg16::IY, (operator_add, "ADD"))),
                0x8e => Some(build_operator_a_idx_offset(Reg16::IY, (operator_adc, "ADC"))),
                0x96 => Some(build_operator_a_idx_offset(Reg16::IY, (operator_sub, "SUB"))),
                0x9e => Some(build_operator_a_idx_offset(Reg16::IY, (operator_sbc, "SBC"))),
                0xa6 => Some(build_operator_a_idx_offset(Reg16::IY, (operator_and, "ADD"))),
                0xae => Some(build_operator_a_idx_offset(Reg16::IY, (operator_xor, "XOR"))),
                0xb6 => Some(build_operator_a_idx_offset(Reg16::IY, (operator_or, "OR"))),
                0xbe => Some(build_operator_a_idx_offset(Reg16::IY, (operator_cp, "CP"))),

                _ => None
            };
            self.prefix_fd[c as usize] = opcode;
        }
    }

    fn load_no_prefix(&mut self) {
        for c in 0..=255 {
            let p = DecodingHelper::parts(c);
            let opcode = match p.x {
                0 => match p.z {
                    0 => match p.y { // Relative jumps and assorted ops.
                        0 => Some(build_nop()), // NOP
                        1 => Some(build_ex_af()), // EX AF, AF'
                        2 => Some(build_djnz()), // DJNZ d
                        3 => Some(build_jr_unconditional()), // JR d
                        4..=7 => Some(build_jr_eq(CC[p.y-4])),
                        _ => panic!("Unreachable")
                    },
                    1 => match p.q {
                        0 =>  Some(build_ld_rr_nn(RP[p.p])), // LD rr, nn -- 16-bit load add
                        1 =>  Some(build_add_hl_rr(RP[p.p])), // ADD HL, rr -- 16-bit add
                        _ => panic!("Unreachable")
                    },
                    2 => match p.q {
                        0 =>  match p.p {
                            0 => Some(build_ld_prr_a(Reg16::BC)), // LD (BC), A
                            1 => Some(build_ld_prr_a(Reg16::DE)), // LD (DE), A
                            2 => Some(build_ld_pnn_rr(Reg16::HL, true)), // LD (nn), HL
                            3 => Some(build_ld_pnn_a()), // LD (nn), A
                            _ => panic!("Unreachable")
                        },
                        1 =>  match p.p {
                            0 => Some(build_ld_a_prr(Reg16::BC)), // LD A, (BC)
                            1 => Some(build_ld_a_prr(Reg16::DE)), // LD A, (DE)
                            2 => Some(build_ld_rr_pnn(Reg16::HL, true)), // LD HL, (nn)
                            3 => Some(build_ld_a_pnn()), // LD A, (nn)
                            _ => panic!("Unreachable")
                        }
                        _ => panic!("Unreachable")
                    },
                    3 => match p.q {
                        0 =>  Some(build_inc_dec_rr(RP[p.p], true)), // INC rr -- 16-bit inc
                        1 =>  Some(build_inc_dec_rr(RP[p.p], false)), // DEC rr -- 16-bit dec
                        _ => panic!("Unreachable")                       
                    },
                    4 => Some(build_inc_r(R[p.y])), // INC r -- 8 bit inc
                    5 => Some(build_dec_r(R[p.y])), // DEC r -- 8 bit dec
                    6 => Some(build_ld_r_n(R[p.y])), // LD r, n -- 8 bit load imm
                    7 => match p.y {
                        0..=3 => Some(build_rot_r(Reg8::A, ROT[p.y], true, false)), // rotA
                        4 => Some(build_daa()), // DAA, decimal adjust A
                        5 => Some(build_cpl()), // CPL, complement adjust A
                        6 => Some(build_scf()), // SCF, set carry flag
                        7 => Some(build_ccf()), // CCF, clear carry flag
                        _ => panic!("Unreachable")
                    },
                    _ => panic!("Unreachable")
                },
                1 => match (p.z, p.y) {
                    (6, 6) => Some(build_halt()), // HALT, exception instead of LD (HL), (HL)
                    _ => Some(build_ld_r_r(R[p.y], R[p.z], false)), // LD r[y], r[z] -- 8 bit load imm
                },
            2 => Some(build_operator_a_r(R[p.z], ALU[p.y])), // alu A, r
            3 => match p.z {
                    0 => Some(build_ret_eq(CC[p.y])), // RET cc
                    1 => match p.q {
                        0 => Some(build_pop_rr(RP2[p.p])), // POP rr
                        1 => match p.p {
                            0 => Some(build_ret()), // RET
                            1 => Some(build_exx()), // EXX
                            2 => Some(build_jp_hl()), // JP HL
                            3 => Some(build_ld_sp_hl()), // LD SP, HL
                            _ => panic!("Unreachable")
                        },
                        _ => panic!("Unreachable")
                    },
                    2 => Some(build_jp_eq(CC[p.y])), // JP cc, nn
                    3 => match p.y {
                        0 => Some(build_jp_unconditional()), // JP nn
                        1 => None, // CB prefix
                        2 => Some(build_out_n_a()),  // OUT (n), A
                        3 => Some(build_in_a_n()),   // IN A, (n)
                        4 => Some(build_ex_psp_hl()), // EX (SP), HL
                        5 => Some(build_ex_de_hl()),  // EX DE, HL
                        6 => Some(build_conf_interrupts(false)), // DI
                        7 => Some(build_conf_interrupts(true)),  // EI
                        _ => panic!("Unreachable")
                    }
                    4 => Some(build_call_eq(CC[p.y])),
                    5 => match p.q {
                        0 => Some(build_push_rr(RP2[p.p])), // PUSH rr
                        1 => match p.p {
                            0 => Some(build_call()), // Call nn
                            1 => None, // DD prefix
                            2 => None, // ED prefix
                            3 => None, // FD prefix
                            _ => panic!("Unreachable")
                        },
                        _ => panic!("Unreachable")
                    },
                    6 => Some(build_operator_a_n(ALU[p.y])), // alu A, n
                    7 => Some(build_rst(p.y as u8 * 8)), // RST
                    _ => panic!("Unreachable")
                    },
                _ => panic!("Unreachable")
            };
/*
            match opcode.as_ref() {
                None => println!("0x{:02x} {:20}: {:?}", c, "Pending", p),
                Some(o) => println!("0x{:02x} {:20}: {:?}", c, o.name, p)
            }
*/
            self.no_prefix[c as usize] = opcode;
        }
    }

    fn load_prefix_cb(&mut self) {
        for c in 0..=255 {
            let p = DecodingHelper::parts(c);
            let opcode = match p.x {
                0 => Some(build_rot_r(R[p.z], ROT[p.y], false, false)), // Shifts
                1 => Some(build_bit_r(p.y as u8, R[p.z])), // BIT
                2 => Some(build_set_res_r(p.y as u8, R[p.z], false)), // RES
                3 => Some(build_set_res_r(p.y as u8, R[p.z], true)), // SET
                _ => panic!("Unreachable")
            };

/*
            match opcode.as_ref() {
                None => println!("0x{:02x} 0x{:02x} {:15}: {:?}", 0xcb, c, "Pending", p),
                Some(o) => println!("0x{:02x} 0x{:02x} {:15}: {:?}", 0xcb, c, o.name, p)
            }
*/
            self.prefix_cb[c as usize] = opcode;
        }
    }

    fn load_prefix_cb_indexed(&mut self) {
        for c in 0..=255 {
            let p = DecodingHelper::parts(c);
            let opcode = match p.x {
                0 => Some(build_rot_r(R[p.z], ROT[p.y], false, true)), // Shifts
                1 => Some(build_bit_r(p.y as u8, R[p.z])), // BIT
                2 => Some(build_indexed_set_res_r(p.y as u8, R[p.z], false)), // RES
                3 => Some(build_indexed_set_res_r(p.y as u8, R[p.z], true)), // SET
                _ => panic!("Unreachable")
            };

/*
            match opcode.as_ref() {
                None => println!("0x{:02x} 0x{:02x} {:15}: {:?}", 0xcb, c, "Pending", p),
                Some(o) => println!("0x{:02x} 0x{:02x} {:15}: {:?}", 0xcb, c, o.name, p)
            }
*/
            self.prefix_cb_indexed[c as usize] = opcode;
        }
    }


    fn load_prefix_ed(&mut self) {
        for c in 0..=255 {
            let p = DecodingHelper::parts(c);
            let opcode = match p.x {
                0 => match p.z {
                    0 => match p.y {
                        0 | 1 | 2 | 3 | 4 | 5 | 7 => Some(build_in0_r_n(R[p.y])),
                        _ => Some(build_noni_nop()),
                    }
                    1 => match p.y {
                        6 => Some(build_ld_rr_ind_hl(Reg16::IY)),
                        _ => Some(build_out0_n_r(R[p.y])),
                    }
                    2 => match p.p {
                        0 | 1 | 2 => Some(build_lea_rr_ind_offset(RP[p.p], Reg16::IX)),
                        3 => Some(build_lea_rr_ind_offset(Reg16::IX, Reg16::IX)),
                        _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                    },
                    3 => match p.p {
                        0 | 1 | 2 => Some(build_lea_rr_ind_offset(RP[p.p], Reg16::IY)),
                        3 => Some(build_lea_rr_ind_offset(Reg16::IY, Reg16::IY)),
                        _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                    },
                    4 => Some(build_tst_a_r(R[p.y])),
                    6 => match p.y {
                        7 => Some(build_ld_ind_hl_rr(Reg16::IY)),
                        _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                    }
                    7 => match p.y {
                        0 | 2 | 4 => Some(build_ld_rr_ind_hl(RP[p.p])),
                        1 | 3 | 5 => Some(build_ld_ind_hl_rr(RP[p.p])),
                        6 => Some(build_ld_rr_ind_hl(Reg16::IX)),
                        7 => Some(build_ld_ind_hl_rr(Reg16::IX)),
                        _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                    },
                    _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                },
                1 => match p.z {
                    0 => match p.y {
                        6 => Some(build_in_0_c()), // IN (C)
                        _ => Some(build_in_r_c(R[p.y])), // IN r, (C)
                    }
                    1 => match p.y {
                        6 => Some(build_out_c_0()), // OUT (C), 0
                        _ => Some(build_out_c_r(R[p.y])), // OUT (C), r
                    }
                    2 => match p.q {
                        0 => Some(build_sbc_hl_rr(RP[p.p])), // SBC HL, rr
                        1 => Some(build_adc_hl_rr(RP[p.p])), // ADC HL, rr
                        _ => panic!("Unreachable")
                    },
                    3 => match p.q {
                        0 => Some(build_ld_pnn_rr(RP[p.p], false)), // LD (nn), rr -- 16 bit loading
                        1 => Some(build_ld_rr_pnn(RP[p.p], false)), // LD rr, (nn) -- 16 bit loading
                        _ => panic!("Unreachable")
                    },
                    4 => match p.y {
                        1 | 3 | 5 | 7 => Some(build_mlt_rr(RP[p.p])),
                        2 => Some(build_lea_rr_ind_offset(Reg16::IX, Reg16::IY)),
                        4 => Some(build_tst_a_n()),
                        6 => Some(build_log_unimplemented("0x74: TSTIO n")),
                        _ => Some(build_neg()), // NEG
                    },
                    5 => match p.y {
                        1 => Some(build_reti()), // RETI
                        2 => Some(build_lea_rr_ind_offset(Reg16::IY, Reg16::IX)),
                        4 => Some(build_pea(Reg16::IX)),
                        5 => Some(build_ld_mb_a()),
                        7 => Some(build_stmix()),
                        _ => Some(build_retn())  // RETN
                    }
                    6 => match p.y {
                        4 => Some(build_pea(Reg16::IY)),
                        5 => Some(build_ld_a_mb()),
                        6 => Some(build_log_unimplemented("SLP")), // 0x76
                        7 => Some(build_rsmix()),
                        _ => Some(build_im(IM[p.y])) // IM #
                    }
                    7 => match p.y {
                        0 => Some(build_ld_r_r(Reg8::I, Reg8::A, true)), // LD I, A
                        1 => Some(build_ld_r_r(Reg8::R, Reg8::A, true)), // LD R, A
                        2 => Some(build_ld_r_r(Reg8::A, Reg8::I, true)), // LD A, I
                        3 => Some(build_ld_r_r(Reg8::A, Reg8::R, true)), // LD A, R
                        4 => Some(build_rxd(ShiftDir::Right, "RRD")), // RRD
                        5 => Some(build_rxd(ShiftDir::Left, "RLD")),  // RLD
                        6 => Some(build_nop()), // NOP
                        7 => Some(build_nop()), // NOP
                        _ => panic!("Unreacheable")
                    },
                    _ => panic!("Unreacheable")
                },
                2 =>
                    if p.z <= 3 && p.y >= 4 {
                        // Table "bli"
                        match p.z {
                            0 => Some(build_ld_block( BLI_A[p.y-4])), // Block LDxx
                            1 => Some(build_cp_block( BLI_A[p.y-4])), // Block CPxx
                            2 => Some(build_in_block( BLI_A[p.y-4])), // Block INxx
                            3 => Some(build_out_block(BLI_A[p.y-4])), // Block OUTxx
                            _ => panic!("Unreacheable")
                        }
                    } else if p.z == 3 {
                        match p.y {
                            0 => Some(build_log_unimplemented("OTIM")), // 0x83
                            1 => Some(build_log_unimplemented("OTDM")), // 0x8b
                            2 => Some(build_log_unimplemented("OTIMR")), // 0x93
                            3 => Some(build_log_unimplemented("OTDMR")), // 0x9b
                            _ => Some(build_noni_nop()),
                        }
                    } else if p.z == 4 {
                        match p.y {
                            0 => Some(build_log_unimplemented("INI2")), // 0x84
                            1 => Some(build_log_unimplemented("IND2")), // 0x8c
                            2 => Some(build_log_unimplemented("INI2R")), // 0x94
                            3 => Some(build_log_unimplemented("IND2R")), // 0x9c
                            4 => Some(build_log_unimplemented("OUTI2")), // 0xa4
                            5 => Some(build_log_unimplemented("OUTD2")), // 0xac
                            6 => Some(build_log_unimplemented("OUTI2R")), // 0xb4
                            7 => Some(build_log_unimplemented("OTD2R")), // 0xbc
                            _ => Some(build_noni_nop()),
                        }
                    } else {
                        Some(build_noni_nop()) // NONI + NOP
                    },
                3 => match p.z {
                    2 => match p.y {
                        0 => Some(build_log_unimplemented("INIRX")), // 0xc2
                        1 => Some(build_log_unimplemented("INDRX")), // 0xca
                        _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                    }
                    3 => match p.y {
                        0 => Some(build_otirx_or_otdrx(true /* otirx */)), // 0xc3
                        1 => Some(build_otirx_or_otdrx(false /* otdrx */)), // 0xcb
                        _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                    }
                    7 => match p.y {
                        0 => Some(build_log_unimplemented("ld i,hl")),
                        2 => Some(build_log_unimplemented("ld hl,i")),
                        _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                    },
                    _ => Some(build_noni_nop()), // Invalid instruction NONI + NOP
                },
                _ => panic!("Unreachable")
            };

/*
            match opcode.as_ref() {
                None => println!("0x{:02x} 0x{:02x} {:15}: {:?}", 0xed, c, "Pending", p),
                Some(o) => println!("0x{:02x} 0x{:02x} {:15}: {:?}", 0xed, c, o.name, p)
            }
*/
            self.prefix_ed[c as usize] = opcode;
        }
    }

    fn load_has_displacement(&mut self) {
        self.has_displacement[0x34] = true;
        self.has_displacement[0x35] = true;
        self.has_displacement[0x36] = true;
        self.has_displacement[0x46] = true;
        self.has_displacement[0x4e] = true;
        self.has_displacement[0x56] = true;
        self.has_displacement[0x5e] = true;
        self.has_displacement[0x66] = true;
        self.has_displacement[0x6e] = true;
        self.has_displacement[0x70] = true;
        self.has_displacement[0x71] = true;
        self.has_displacement[0x72] = true;
        self.has_displacement[0x73] = true;
        self.has_displacement[0x74] = true;
        self.has_displacement[0x75] = true;
        self.has_displacement[0x77] = true;
        self.has_displacement[0x7e] = true;
        self.has_displacement[0x86] = true;
        self.has_displacement[0x8e] = true;
        self.has_displacement[0x96] = true;
        self.has_displacement[0x9e] = true;
        self.has_displacement[0xa6] = true;
        self.has_displacement[0xae] = true;
        self.has_displacement[0xb6] = true;
        self.has_displacement[0xbe] = true;
    }
}

#[derive(Debug)]
struct DecodingHelper {
    // See notation in http://www.z80.info/decoding.htm    
    x: usize,
    y: usize,
    z: usize,
    p: usize,
    q: usize
}

impl DecodingHelper {
    fn parts(code: u8) -> DecodingHelper {
        DecodingHelper {
            x: (code >> 6) as usize,
            y: ((code >> 3) & 7) as usize,
            z: (code & 7) as usize,
            p: ((code >> 4) & 3) as usize,
            q: ((code >> 3) & 1) as usize,
        }
    }
}


pub const RP:  [Reg16; 4] = [Reg16::BC, Reg16::DE, Reg16::HL, Reg16::SP];
pub const RP2: [Reg16; 4] = [Reg16::BC, Reg16::DE, Reg16::HL, Reg16::AF];
pub const R:  [Reg8; 8] = [Reg8::B, Reg8::C, Reg8::D, Reg8::E, Reg8::H, Reg8::L, Reg8::_HL, Reg8::A];
pub const IM: [u8; 8] = [0, 0, 1, 2, 0, 0, 1, 2];

pub const CC: [(Flag, bool, &str); 8] = [
    (Flag::Z, false, "NZ"),
    (Flag::Z, true,  "Z"),
    (Flag::C, false, "NC"),
    (Flag::C, true,  "C"),
    (Flag::P, false, "PO"),
    (Flag::P, true,  "PE"),
    (Flag::S, false, "P"),
    (Flag::S, true,  "M")
];

pub const ROT: [(ShiftDir, ShiftMode, &str); 8] = [
    (ShiftDir::Left,  ShiftMode::RotateCarry, "RLC"),
    (ShiftDir::Right, ShiftMode::RotateCarry, "RRC"),
    (ShiftDir::Left,  ShiftMode::Rotate,      "RL" ),
    (ShiftDir::Right, ShiftMode::Rotate,      "RR" ),
    (ShiftDir::Left,  ShiftMode::Arithmetic,  "SLA"),
    (ShiftDir::Right, ShiftMode::Arithmetic,  "SRA"),
    (ShiftDir::Left,  ShiftMode::Logical,     "SLL"),
    (ShiftDir::Right, ShiftMode::Logical,     "SRL"),
];

//pub const ALU: [(fn(&mut State, u8, u8) -> u8, &'static str); 8] = [
pub const ALU: [(Operator, &str); 8] = [
    (operator_add, "ADD"),
    (operator_adc, "ADC"),
    (operator_sub, "SUB"),
    (operator_sbc, "SBC"),
    (operator_and, "AND"),
    (operator_xor, "XOR"),
    (operator_or,  "OR"),
    (operator_cp,  "CP")
];

pub const BLI_A: [(bool, bool, &str); 4] = [
    (true,  false, "I"),
    (false, false, "D"),
    (true,  true, "IR"),
    (false, true, "DR")
];

pub fn build_log_unimplemented(name: &'static str) -> Opcode {
    Opcode {
        name: name.to_string(),
        action: Box::new(move |_: &mut Environment| {
            println!("Unimplemented opcode: {}", name);
        })
    }
}

