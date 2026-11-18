use ez80::{Cpu, Flag, Machine, PlainMachine, Reg16};

const NMI_VECTOR: u32 = 0x000066;

fn step(cpu: &mut Cpu, machine: &mut PlainMachine) {
    if !cpu.accept_pending_ez80_nmi(machine) {
        cpu.fast_execute_instruction(machine);
    }
}

fn load(machine: &mut PlainMachine, address: u32, bytes: &[u8]) {
    for (offset, &byte) in bytes.iter().enumerate() {
        machine.poke(address + offset as u32, byte);
    }
}

#[test]
fn pending_nmi_vectors_before_normal_fetch_under_di() {
    let mut machine = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_adl(true);
    cpu.state.reg.set24(Reg16::SP, 0x006000);
    cpu.state.set_pc(0x001200);
    load(&mut machine, 0x001200, &[0xf3, 0x3c]); // DI; INC A
    cpu.state.reg.set_a(0x2a);
    cpu.fast_execute_instruction(&mut machine);
    assert!(!cpu.state.reg.get_iff1());
    assert_eq!(cpu.state.pc(), 0x001201);
    let retired = cpu.state.instructions_executed;

    cpu.signal_nmi();
    step(&mut cpu, &mut machine);

    assert_eq!(cpu.state.pc(), NMI_VECTOR, "NMI vector before normal fetch");
    assert_eq!(cpu.state.reg.a(), 0x2a);
    assert_eq!(cpu.state.instructions_executed, retired);
    assert!(!cpu.state.nmi_pending);
}

#[test]
fn nmi_wakes_halt_and_retn_resumes_the_next_instruction() {
    let mut machine = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_adl(true);
    cpu.state.reg.set24(Reg16::SP, 0x006000);
    cpu.state.set_pc(0x001200);
    load(&mut machine, 0x001200, &[0xf3, 0x76, 0x3c]); // DI; HALT; INC A
    load(&mut machine, NMI_VECTOR, &[0xed, 0x45]); // RETN
    cpu.state.reg.set_a(0x2a);
    step(&mut cpu, &mut machine);
    step(&mut cpu, &mut machine);
    assert!(cpu.is_halted());
    assert_eq!(cpu.state.pc(), 0x001202);

    cpu.signal_nmi();
    step(&mut cpu, &mut machine);
    assert_eq!(cpu.state.pc(), NMI_VECTOR);
    assert!(!cpu.is_halted());
    step(&mut cpu, &mut machine);
    assert_eq!(cpu.state.pc(), 0x001202);
    assert_eq!(cpu.state.sp(), 0x006000);
    assert!(!cpu.state.reg.get_iff1());
    step(&mut cpu, &mut machine);
    assert_eq!(cpu.state.reg.a(), 0x2b);
}

#[test]
fn nmi_frames_and_returns_match_all_four_adl_madl_combinations() {
    const MBASE: u8 = 1;
    const SPS: u32 = 0x017000;
    const SPL: u32 = 0x036000;

    for (adl, madl, frame) in [
        (false, false, &[0x34, 0x12][..]),
        (true, false, &[0x34, 0x12, 0x02][..]),
        (false, true, &[0x02, 0x34, 0x12][..]),
        (true, true, &[0x03, 0x34, 0x12, 0x02][..]),
    ] {
        for enabled in [false, true] {
            let mut machine = PlainMachine::new();
            let mut cpu = Cpu::new_ez80();
            cpu.set_adl(adl);
            cpu.state.reg.madl = madl;
            cpu.state.reg.mbase = MBASE;
            cpu.state.reg.set16(Reg16::SP, SPS as u16);
            cpu.state.reg.set24(Reg16::SP, SPL);

            let pc = if adl { 0x021234 } else { 0x011234 };
            cpu.state.set_pc(pc - 1);
            machine.poke(pc - 1, if enabled { 0xfb } else { 0xf3 }); // EI or DI
            step(&mut cpu, &mut machine);
            assert_eq!(cpu.state.pc(), pc);
            assert_eq!(cpu.state.reg.get_iff1(), enabled);

            for top in [SPS, SPL] {
                load(&mut machine, top - 5, &[0xa9; 6]);
            }
            let (vector, top, other) = if adl || madl {
                (NMI_VECTOR, SPL, SPS)
            } else {
                (0x010066, SPS, SPL)
            };
            let handler = if madl {
                &[0xed, 0x57, 0x5b, 0xed, 0x45][..] // LD A,I; RETN.L
            } else {
                &[0xed, 0x57, 0xed, 0x45][..] // LD A,I; RETN
            };
            load(&mut machine, vector, handler);
            let retired = cpu.state.instructions_executed;
            cpu.signal_nmi();
            step(&mut cpu, &mut machine);

            let sp = top - frame.len() as u32;
            assert_eq!(cpu.state.pc(), vector);
            assert_eq!(cpu.state.reg.adl, adl || madl);
            assert_eq!(cpu.state.reg.madl, madl);
            assert!(!cpu.state.reg.get_iff1());
            assert_eq!(cpu.state.instructions_executed, retired);
            assert_eq!(cpu.state.sp(), sp);
            for (offset, &byte) in frame.iter().enumerate() {
                assert_eq!(machine.peek(sp + offset as u32), byte);
            }
            assert_eq!(machine.peek(sp - 1), 0xa9);
            assert_eq!(machine.peek(top), 0xa9);
            assert!((0..6).all(|offset| machine.peek(other - offset) == 0xa9));

            step(&mut cpu, &mut machine); // LD A,I copies IFF2 to P/V.
            assert_eq!(cpu.state.reg.get_flag(Flag::P), enabled);
            assert!(!cpu.state.reg.get_iff1());
            step(&mut cpu, &mut machine); // RETN or RETN.L
            assert_eq!(cpu.state.pc(), pc);
            assert_eq!(cpu.state.reg.adl, adl);
            assert_eq!(cpu.state.reg.madl, madl);
            assert_eq!(cpu.state.reg.mbase, MBASE);
            assert_eq!(cpu.state.reg.get_iff1(), enabled);
            assert_eq!(cpu.state.reg.get16(Reg16::SP), SPS as u16);
            assert_eq!(cpu.state.reg.get24(Reg16::SP), SPL);
        }
    }
}

#[test]
fn pending_reset_takes_priority_over_nmi_entry() {
    let mut machine = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_adl(true);
    cpu.state.set_pc(0x001234);
    cpu.state.reg.set24(Reg16::SP, 0x006000);
    load(&mut machine, 0x005ffd, &[0xa9; 3]);
    cpu.signal_reset();
    cpu.signal_nmi();

    assert!(!cpu.accept_pending_ez80_nmi(&mut machine));
    assert!(cpu.state.reset_pending);
    assert!(cpu.state.nmi_pending);
    assert_eq!(cpu.state.pc(), 0x001234);
    assert_eq!(cpu.state.sp(), 0x006000);
    assert!((0x005ffd..0x006000).all(|address| machine.peek(address) == 0xa9));
}

#[test]
fn machine_reset_initializes_cpu_state_and_keeps_the_ez80_decoder() {
    let mut machine = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();
    cpu.set_adl(true);
    cpu.state.reg.madl = true;
    cpu.state.reg.mbase = 1;
    cpu.state.reg.iff1 = true;
    cpu.state.reg.iff2 = true;
    cpu.state.set_pc(0x011200);
    machine.poke(0x011200, 0x76); // HALT
    cpu.fast_execute_instruction(&mut machine);
    assert!(cpu.is_halted());
    cpu.signal_reset();
    cpu.signal_nmi();
    let retired = cpu.state.instructions_executed;

    cpu.reset_ez80_state();

    assert_eq!(cpu.state.pc(), 0);
    assert_eq!(cpu.state.reg.mbase, 0);
    assert!(!cpu.state.reg.adl);
    assert!(!cpu.state.reg.madl);
    assert!(!cpu.state.reg.iff1);
    assert!(!cpu.state.reg.iff2);
    assert!(!cpu.state.reset_pending);
    assert!(!cpu.state.nmi_pending);
    assert!(!cpu.is_halted());
    assert_eq!(cpu.state.instructions_executed, retired);

    load(&mut machine, 0, &[0xed, 0x7d]); // eZ80 STMIX
    cpu.fast_execute_instruction(&mut machine);
    assert!(cpu.state.reg.madl);
    assert_eq!(cpu.state.pc(), 2);
    assert_eq!(cpu.state.instructions_executed, retired + 1);
}
