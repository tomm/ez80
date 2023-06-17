use ez80::*;

fn test_disasm_z80(code: &[u8], expected: &str) {
    let mut sys = PlainMachine::new();
    let mut cpu = Cpu::new_ez80();

    for i in 0..code.len() {
        sys.poke(i as u32, code[i]);
    }

    let disasm = cpu.disasm_instruction(&mut sys);
    assert_eq!(expected, disasm);
}

#[test]
fn test_disasm_ld_hl_ix_n() {
    test_disasm_z80(&[0xdd, 0x27, 0x18], "LD HL, (IX+$18)");
}

#[test]
fn test_disasm_ld_ix_n_hl() {
    test_disasm_z80(&[0xdd, 0x2f, 0x18], "LD (IX+$18), HL");
}

#[test]
fn test_disasm_push_ix() {
    test_disasm_z80(&[0xdd, 0xe5], "PUSH IX");
}

#[test]
fn test_disasm_suffix() {
    test_disasm_z80(&[0x5b, 0xdd, 0xe5], "PUSH.LIL IX");
}

#[test]
fn test_disasm_push_hl() {
    test_disasm_z80(&[0xe5], "PUSH HL");
}
