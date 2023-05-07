use iz80::*;

/*
8080/8085 CPU Exerciser by Ian Bartholomew and Frank Cringles
*/

static CODE: &'static [u8] = include_bytes!("res/8080EX1.COM");

#[test]
#[ignore]
fn test_ex8080() {
    let mut machine = PlainMachine::new();
    let mut cpu = Cpu::new_8080();

    // Load program
    let code = CODE;
    let size = code.len();
    for i in 0..size {
        machine.poke(0x100 + i as u32, code[i]);
    }

    /*
    System call 5

    .org $5
        ret
    */
    //let code = [0xD3, 0x00, 0xC9];
    let code = [0xC9];
    for i in 0..code.len() {
        machine.poke(5 + i as u32, code[i]);
    }

    // Patch to run a single test
    let run_single_test = false;
    let single_test = 3;
    if run_single_test {
        let mut test_start = machine._peek16(0x0120);
        test_start += single_test*2;
        machine._poke16(0x0120, test_start);
        machine._poke16(test_start as u32 + 2 , 0);
    
    }

    cpu.state.set_pc(0x100);
    let trace = false;
    cpu.set_trace(trace);
    let mut tests_passed = 0;
    loop {
        cpu.execute_instruction(&mut machine);

        if trace && false {
            // Test state
            let addr = 0x1d80 as u32;
            print!("Zex state 0x{:04x}: ", addr);
            for i in 0..0x10 {
                print!("{:02x} ", machine.peek(addr + i));
            }
            println!("");
        }

        if cpu.state.pc() == 0x0000 {
            println!("");
            break;
        }

        if cpu.state.pc() == 0x0005 {
            match cpu.registers().get8(Reg8::C) {
                2 => {
                    // C_WRITE
                    print!("{}", cpu.registers().get8(Reg8::E) as char);
                },
                9 => {
                    // C_WRITE_STR
                    let mut address = cpu.registers().get16(Reg16::DE);
                    let mut msg = String::new();
                    loop {
                        let ch = machine.peek(address as u32) as char;
                        address += 1;
                
                        if ch == '$'{
                            break;
                        }
                        msg.push(ch);
                    }
                    if msg.contains("OK") {
                        tests_passed += 1;
                    }
                    print!("{}", msg);
                },
                _ => panic!("BDOS command not implemented")
            }
        }
    }

    if run_single_test {
        assert_eq!(1, tests_passed);
    } else {
        assert_eq!(25, tests_passed);
    }
}
