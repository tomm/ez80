use ez80::*;

#[test]
fn test_ez80_z80() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();

    sys.poke(0x0000, 0x01);  // LD BC, $3456 ; db $12
    sys.poke(0x0001, 0x56); 
    sys.poke(0x0002, 0x34); 
    sys.poke(0x0003, 0x12); 
    cpu.registers().set16(Reg16::BC, 0x0000);

    cpu.execute_instruction(&mut sys);
    assert_eq!(0x3456, cpu.registers().get16(Reg16::BC));
    assert_eq!(0x3, cpu.state.pc());

    //assert_eq!(0x123456, cpu.registers().get24(Reg24::BC));
}

#[test]
fn test_ez80_adl() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_adl(true);

    sys.poke(0x0000, 0x01);  // LD BC, $123456
    sys.poke(0x0001, 0x56); 
    sys.poke(0x0002, 0x34); 
    sys.poke(0x0003, 0x12); 
    cpu.registers().set24(Reg16::BC, 0x0000);

    cpu.execute_instruction(&mut sys);
    assert_eq!(0x123456, cpu.registers().get24(Reg16::BC));
    assert_eq!(0x4, cpu.state.pc());
}

#[test]
fn test_ez80_mem_wrap() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();

    cpu.registers().set16(Reg16::SP, 0xFFFF);
    sys.poke(0x00000, 0xc1); // POP BC
    sys.poke(0x0FFFF, 0xfe); // POP BC
    sys.poke(0x10000, 0xca); // POP BC

    cpu.execute_instruction(&mut sys);
    assert_eq!(0x0001, cpu.registers().get16(Reg16::SP));
    assert_eq!(0x00c1fe, cpu.registers().get24(Reg16::BC));

    cpu.set_adl(true);
    cpu.state.set_pc(0);
    cpu.registers().set24(Reg16::BC, 0);
    cpu.registers().set24(Reg16::SP, 0xFFFF);

    cpu.execute_instruction(&mut sys);
    assert_eq!(0x10002, cpu.registers().get24(Reg16::SP));
    assert_eq!(0xcafe, cpu.registers().get24(Reg16::BC));
}

#[test]
fn test_ez80_pc_wrap() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);

    cpu.registers().set8(Reg8::A, 0);
    cpu.state.reg.mbase = 1;
    cpu.state.set_pc(0xffff);
    sys.poke(0x1FFFF, 0x3c); // INC A
    sys.poke(0x20000, 0x3c); // INC A
    // execute an inc (at 0xffff) and a nop (at 0x0)
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);

    assert_eq!(0x10001, cpu.state.pc());
    assert_eq!(0x01, cpu.registers().get8(Reg8::A));

    cpu.set_adl(true);

    cpu.registers().set8(Reg8::A, 0);
    cpu.state.set_pc(0x1ffff);
    sys.poke(0x1FFFF, 0x3c); // INC A
    sys.poke(0x20000, 0x3c); // INC A
    // execute an inc (at 0xffff) and a nop (at 0x0)
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);

    assert_eq!(0x20001, cpu.state.pc());
    assert_eq!(0x02, cpu.registers().get8(Reg8::A));
}

#[test]
fn test_size_suffixes() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);

    for adl in [false, true] {
        cpu.set_adl(adl);

        cpu.state.set_pc(0);
        cpu.registers().set24(Reg16::DE, 0);
        cpu.registers().set24(Reg16::HL, 0);
        cpu.registers().set24(Reg16::IX, 0);
        cpu.registers().set24(Reg16::IY, 0);

        sys.poke(0x0000, 0x5b); // .LIL
        sys.poke(0x0001, 0x21); // ld.lil hl, $123456
        sys.poke(0x0002, 0x56);
        sys.poke(0x0003, 0x34);
        sys.poke(0x0004, 0x12);
        sys.poke(0x0005, 0x40); // .SIS
        sys.poke(0x0006, 0x11); // ld.sis de, $789a
        sys.poke(0x0007, 0x9a);
        sys.poke(0x0008, 0x78);
        sys.poke(0x0009, 0x49); // .LIS
        sys.poke(0x000a, 0xdd); // ld.lis ix, $1234
        sys.poke(0x000b, 0x21);
        sys.poke(0x000c, 0x34);
        sys.poke(0x000d, 0x12);
        sys.poke(0x000e, 0x52); // SIL
        sys.poke(0x000f, 0xfd); // ld.sil iy, $789abc
        sys.poke(0x0010, 0x21);
        sys.poke(0x0011, 0xbc);
        sys.poke(0x0012, 0x9a);
        sys.poke(0x0013, 0x78);
        sys.poke(0x0014, 0xff);

        cpu.execute_instruction(&mut sys);
        cpu.execute_instruction(&mut sys);
        cpu.execute_instruction(&mut sys);
        cpu.execute_instruction(&mut sys);

        assert_eq!(0x0014, cpu.state.pc());
        assert_eq!(0x123456, cpu.registers().get24(Reg16::HL));
        // note we are assuming the top byte of the register, when
        // using .s prefix, is zero. the actual spec says "undefined"
        assert_eq!(0x789a, cpu.registers().get24(Reg16::DE));
        assert_eq!(0x1234, cpu.registers().get24(Reg16::IX));
        assert_eq!(0x9abc, cpu.registers().get24(Reg16::IY));
    }
}

#[test]
fn test_madl() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);

    sys.poke(0x0000, 0xed); // STMIX
    sys.poke(0x0001, 0x7d);
    sys.poke(0x0002, 0xed); // RSMIX
    sys.poke(0x0003, 0x7e);

    assert_eq!(false, cpu.state.reg.madl);
    cpu.execute_instruction(&mut sys);
    assert_eq!(true, cpu.state.reg.madl);
    cpu.execute_instruction(&mut sys);
    assert_eq!(false, cpu.state.reg.madl);
}

#[test]
fn test_ld_mb() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);
    cpu.set_adl(true);

    cpu.state.reg.mbase = 2;
    cpu.state.reg.set8(Reg8::A, 0);

    sys.poke(0x0000, 0xed); // LD A,MB
    sys.poke(0x0001, 0x6e);
    sys.poke(0x0002, 0x3c); // INC A
    sys.poke(0x0003, 0xed); // LD MB,A
    sys.poke(0x0004, 0x6d);

    assert_eq!(2, cpu.state.reg.mbase);
    cpu.execute_instruction(&mut sys);
    assert_eq!(2, cpu.registers().get8(Reg8::A));
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    assert_eq!(3, cpu.state.reg.mbase);
}

#[test]
fn test_tst_a_hl() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);
    cpu.set_adl(true);

    sys.poke(0x0000, 0xed); // TST A,(HL)
    sys.poke(0x0001, 0x34);

    cpu.state.reg.set8(Reg8::A, 0);
    cpu.state.reg.set24(Reg16::HL, 0);
    cpu.state.set_pc(0);

    cpu.execute_instruction(&mut sys);

    assert_eq!(2, cpu.state.pc());
    assert_eq!(0, cpu.registers().a());
    assert_eq!(false, cpu.registers().get_flag(Flag::C));
    assert_eq!(false, cpu.registers().get_flag(Flag::N));
    assert_eq!(false, cpu.registers().get_flag(Flag::S));

    cpu.state.reg.set8(Reg8::A, 0xff);
    cpu.state.reg.set24(Reg16::HL, 0);
    cpu.state.set_pc(0);

    cpu.execute_instruction(&mut sys);

    assert_eq!(2, cpu.state.pc());
    assert_eq!(0xff, cpu.registers().a());
    assert_eq!(false, cpu.registers().get_flag(Flag::C));
    assert_eq!(false, cpu.registers().get_flag(Flag::N));
    assert_eq!(true, cpu.registers().get_flag(Flag::S));
}

#[test]
fn test_tst_a_n() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);
    cpu.set_adl(true);

    sys.poke(0x0000, 0xed); // TST A, 0xed
    sys.poke(0x0001, 0x64);
    sys.poke(0x0002, 0xed);

    cpu.state.reg.set8(Reg8::A, 0);
    cpu.state.set_pc(0);

    cpu.execute_instruction(&mut sys);

    assert_eq!(3, cpu.state.pc());
    assert_eq!(0, cpu.registers().a());
    assert_eq!(false, cpu.registers().get_flag(Flag::C));
    assert_eq!(false, cpu.registers().get_flag(Flag::N));
    assert_eq!(false, cpu.registers().get_flag(Flag::S));

    cpu.state.reg.set8(Reg8::A, 0xff);
    cpu.state.set_pc(0);

    cpu.execute_instruction(&mut sys);

    assert_eq!(3, cpu.state.pc());
    assert_eq!(0xff, cpu.registers().a());
    assert_eq!(false, cpu.registers().get_flag(Flag::C));
    assert_eq!(false, cpu.registers().get_flag(Flag::N));
    assert_eq!(true, cpu.registers().get_flag(Flag::S));
}

#[test]
fn test_tst_a_r() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);
    cpu.set_adl(true);

    sys.poke(0x0000, 0xed); // TST A, B
    sys.poke(0x0001, 0x04);

    cpu.state.reg.set8(Reg8::A, 0);
    cpu.state.reg.set8(Reg8::B, 0xed);
    cpu.state.set_pc(0);

    cpu.execute_instruction(&mut sys);

    assert_eq!(2, cpu.state.pc());
    assert_eq!(0, cpu.registers().a());
    assert_eq!(false, cpu.registers().get_flag(Flag::C));
    assert_eq!(false, cpu.registers().get_flag(Flag::N));
    assert_eq!(false, cpu.registers().get_flag(Flag::S));

    cpu.state.reg.set8(Reg8::A, 0xff);
    cpu.state.set_pc(0);

    cpu.execute_instruction(&mut sys);

    assert_eq!(2, cpu.state.pc());
    assert_eq!(0xff, cpu.registers().a());
    assert_eq!(false, cpu.registers().get_flag(Flag::C));
    assert_eq!(false, cpu.registers().get_flag(Flag::N));
    assert_eq!(true, cpu.registers().get_flag(Flag::S));
}

#[test]
fn test_pea() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);
    cpu.set_adl(true);

    sys.poke(0x0000, 0xed); // PEA IX+$12
    sys.poke(0x0001, 0x65);
    sys.poke(0x0002, 0x12);

    cpu.state.reg.set24(Reg16::IX, 0xabcdef);
    cpu.state.reg.set24(Reg16::SP, 0x100);

    cpu.execute_instruction(&mut sys);

    assert_eq!(3, cpu.state.pc());
    assert_eq!(0xfd, cpu.state.sp());
    assert_eq!(0x01, sys.peek(0xfd));
    assert_eq!(0xce, sys.peek(0xfe));
    assert_eq!(0xab, sys.peek(0xff));
}

#[test]
fn test_alu_ixh_ihl() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);
    cpu.set_adl(true);

    sys.poke(0x0000, 0xaf); // xor a
    sys.poke(0x0001, 0xdd); // ld ix, $cafeba
    sys.poke(0x0002, 0x21);
    sys.poke(0x0003, 0xba);
    sys.poke(0x0004, 0xfe);
    sys.poke(0x0005, 0xca);
    sys.poke(0x0006, 0xdd); // add a,ixh
    sys.poke(0x0007, 0x84);
    sys.poke(0x0008, 0xdd); // add a,ihl
    sys.poke(0x0009, 0x85);

    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);

    assert_eq!(0xcafeba, cpu.state.reg.get24(Reg16::IX));
    assert_eq!(0x0, cpu.state.reg.a());

    cpu.execute_instruction(&mut sys);

    assert_eq!(0xfe, cpu.state.reg.a());

    cpu.execute_instruction(&mut sys);

    assert_eq!(0xb8, cpu.state.reg.a());
}

#[test]
fn test_alu_ld_ixh_ixl() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);
    cpu.set_adl(true);
    cpu.state.reg.set24(Reg16::IX, 0xcafeba);

    sys.poke(0x0000, 0xdd); // ld ixh,$de
    sys.poke(0x0001, 0x26);
    sys.poke(0x0002, 0xde);
    sys.poke(0x0003, 0xdd); // ld ixl,$89
    sys.poke(0x0004, 0x2e);
    sys.poke(0x0005, 0x89);
    sys.poke(0x0006, 0xdd); // dec ixh
    sys.poke(0x0007, 0x25);
    sys.poke(0x0008, 0xdd); // inc ixh
    sys.poke(0x0009, 0x24);

    cpu.execute_instruction(&mut sys);
    assert_eq!(0xcadeba, cpu.state.reg.get24(Reg16::IX));
    cpu.execute_instruction(&mut sys);
    assert_eq!(0xcade89, cpu.state.reg.get24(Reg16::IX));
    cpu.execute_instruction(&mut sys);
    assert_eq!(0xcadd89, cpu.state.reg.get24(Reg16::IX));
    cpu.execute_instruction(&mut sys);
    assert_eq!(0xcade89, cpu.state.reg.get24(Reg16::IX));
}

#[test]
fn test_24bit_alu_flags() {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_trace(true);

    // 16-bit cp hl,de
    sys.poke(0x0000, 0x0); // nop
    sys.poke(0x0001, 0xed); // sbc hl,de
    sys.poke(0x0002, 0x52);
    sys.poke(0x0003, 0x19); // add hl,de

    cpu.set_adl(true);
    cpu.state.reg.pc = 0;
    cpu.state.reg.set8(Reg8::F, 0);
    cpu.state.reg.set24(Reg16::HL, 0xffffff);
    cpu.state.reg.set24(Reg16::DE, 0x000001);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    assert!(!cpu.state.reg.get_flag(Flag::Z));
    assert!(!cpu.state.reg.get_flag(Flag::C));

    cpu.set_adl(false);
    cpu.state.reg.pc = 0;
    cpu.state.reg.set8(Reg8::F, 0);
    cpu.state.reg.set24(Reg16::HL, 0xffffff);
    cpu.state.reg.set24(Reg16::DE, 0x000001);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    assert!(!cpu.state.reg.get_flag(Flag::Z));
    assert!(!cpu.state.reg.get_flag(Flag::C));

    cpu.set_adl(true);
    cpu.state.reg.pc = 0;
    cpu.state.reg.set8(Reg8::F, 0);
    cpu.state.reg.set24(Reg16::HL, 0xfffffe);
    cpu.state.reg.set24(Reg16::DE, 0xffffff);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    assert!(!cpu.state.reg.get_flag(Flag::Z));
    assert!(cpu.state.reg.get_flag(Flag::C));

    cpu.set_adl(false);
    cpu.state.reg.pc = 0;
    cpu.state.reg.set8(Reg8::F, 0);
    cpu.state.reg.set24(Reg16::HL, 0xfffffe);
    cpu.state.reg.set24(Reg16::DE, 0xffffff);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    assert!(!cpu.state.reg.get_flag(Flag::Z));
    assert!(cpu.state.reg.get_flag(Flag::C));

    cpu.set_adl(true);
    cpu.state.reg.pc = 0;
    cpu.state.reg.set8(Reg8::F, 0);
    cpu.state.reg.set24(Reg16::HL, 0xfffffe);
    cpu.state.reg.set24(Reg16::DE, 0xfffffe);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    assert!(cpu.state.reg.get_flag(Flag::Z));
    assert!(!cpu.state.reg.get_flag(Flag::C));

    cpu.set_adl(false);
    cpu.state.reg.pc = 0;
    cpu.state.reg.set8(Reg8::F, 0);
    cpu.state.reg.set24(Reg16::HL, 0xfffffe);
    cpu.state.reg.set24(Reg16::DE, 0xfffffe);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    cpu.execute_instruction(&mut sys);
    assert!(cpu.state.reg.get_flag(Flag::Z));
    assert!(!cpu.state.reg.get_flag(Flag::C));
}
