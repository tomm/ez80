use iz80::*;

// From https://github.com/raxoft/z80test
// Not passing

//static CODE: &'static [u8] = include_bytes!("res/z80doc.out");
//static CODE: &'static [u8] = include_bytes!("res/z80ccf.out");
//static CODE: &'static [u8] = include_bytes!("res/z80docflags.out");
//static CODE: &'static [u8] = include_bytes!("res/z80flags.out");
//static CODE: &'static [u8] = include_bytes!("res/z80memptr.out");
static CODE: &'static [u8] = include_bytes!("res/z80full.out");

const START: u16 = 0x8000;

#[test]
#[ignore]
fn z80test() {
    let mut cpu = Cpu::new_z80();
    let mut machine = PlainMachine::new();

    // Load program
    let code = CODE;
    let size = code.len();
    for i in 0..size {
        machine.poke(START as u32 + i as u32, code[i]);
    }

    // Do nothing on 0x1601 and RST 0x10
    machine.poke(0x1601, 0xc9); // RET
    machine.poke(0x0010, 0xc9); // RET

    // Patch to run a single test
    let run_single_test = false;
    let single_test = 148;
    if run_single_test {
        machine._poke16(0x802b, single_test); // ld bc, 0 to ld bc, test
        let mut test_start = machine._peek16(0x802e);
        println!("Test table {:x}", test_start);
        test_start += single_test*2;
        println!("Test table {:x}", test_start);
        machine._poke16(0x802e, test_start); // Move start
        machine._poke16(test_start as u32 + 2 , 0); // NUL terminate test
    }

    cpu.state.set_pc(START as u32);
    let trace = false;
    cpu.set_trace(trace);
    let mut msg = String::new();
    loop {
        cpu.execute_instruction(&mut machine);

        if cpu.state.pc() == 0x0000 {
            println!("");
            break;
        }

        if cpu.state.pc() == 0x0010 {
            let mut ch = cpu.registers().get8(Reg8::A) as char;
            if ch == '\r' {
                ch = '\n'
            } else if ch as u8 == 23 {
                ch = ' '
            } else if ch as u8 == 26 {
                ch = ' '
            } 
            //print!("{}[{}]", ch, ch as u8);
            print!("{}", ch);
            msg.push(ch);
        }
    }

    assert_eq!(true, msg.contains("CPU TESTS OK"));
}
