use iz80::*;

const ROM_SIZE: usize = 0x40000; // 256 KiB
const RAM_SIZE: usize = 0x80000; // 512 KiB
const MEM_SIZE: usize = ROM_SIZE + RAM_SIZE;

pub struct AgonMachine {
    mem: [u8; MEM_SIZE],
    io: [u8; 65536]
}

impl AgonMachine {
    /// Returns a new AgonMachine instance
    pub fn new() -> AgonMachine {
        AgonMachine {
            mem: [0; MEM_SIZE],
            io: [0; 65536]
        }
    }
}

impl Default for AgonMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine for AgonMachine {
    fn peek(&self, address: u32) -> u8 {
        self.mem[address as usize]
    }
    fn poke(&mut self, address: u32, value: u8) {
        self.mem[address as usize] = value;
    }

    fn port_in(&mut self, address: u16) -> u8 {
        //println!("IN({:02X}) = 0", address);
        if address == 0xa2 {
            0x0 // UART0 clear to send
        } else if address == 0xc5 {
            0x40
            // UART_LSR_ETX		EQU 	%40
        } else {
            self.io[address as usize]
        }
    }
    fn port_out(&mut self, address: u16, value: u8) {
        //if value < 128 && value > 31 {
        //    println!("{}", char::from_u32(value as u32).unwrap());
        //} else {
            println!("OUT({:02X}) = {:02X}", address, value);
        //}
        //if address == 0xc0 /* UART0_REG_THR */ {
            //println!("OUTPUT to UART0: {:02X}", value);
            //print!("{}", char::from_u32(value as u32).unwrap());
        //}
        self.io[address as usize] = value;
    }
}

fn main() {
    // Prepare the device
    let mut machine = AgonMachine::new();
    let mut cpu = Cpu::new_ez80();
    //cpu.set_trace(true);

    // Load program inline or from a file with:
    let code = include_bytes!("../../../MOS.bin");
    println!("MOS.bin loaded ({} bytes)", code.len());
    for (i, e) in code.iter().enumerate() {
        machine.poke(i as u32, *e);
    }

    // Run emulation
    cpu.state.set_pc(0x0000);
    //for _ in 0..3600 {
    loop {
        if cpu.state.pc() == 0x0 { println!("_reset()") };
        if cpu.state.pc() == 0x0d96 { println!("main()") };
        if cpu.state.pc() == 0x0c80 { println!("wait_ESP32()") };
        if cpu.state.pc() == 0xa966 { println!("open_UART0()") };
        if cpu.state.pc() == 0x2b7f { println!("init_timer0()") };
        if cpu.state.pc() == 0x2c36 { println!("enable_timer0()") };
        if cpu.state.pc() == 0xae16 { println!("__print_sendstring()") };
        if cpu.state.pc() == 0x0909 {
            println!("putch({})", machine.peek(cpu.state.reg.get24(Reg16::SP)+3));
        };
        if cpu.state.pc() == 0x08d4 { println!("UART0_serial_PUTCH()") };
        if cpu.state.pc() == 0x084e { println!("UART0_wait_CTS()") };
        if cpu.state.pc() == 0x0860 { println!("UART0_serial_TX()") };
        if cpu.state.pc() == 0x063a { println!("wait_timer0()") };
        if cpu.state.pc() == 0xb162 { println!("uitoa()") };
        if cpu.state.pc() == 0xb7b5 { println!("itol()") };
        if cpu.state.pc() == 0xb09c { println!("strlen()") };
        if cpu.state.pc() == 0xb184 { println!("ultoa()") };
        if cpu.state.pc() == 0xb418 { println!("lcmpzero()") };
        if cpu.state.pc() == 0xb7bc { println!("lcmpu()") };
        if cpu.state.pc() == 0xadfe { println!("setflag()") };
        if cpu.state.pc() == 0xb7d2 { println!("case8D()") };
        if cpu.state.pc() == 0x2c92 { println!("wait_VDP()") };
        if cpu.state.pc() == 0x2657 { println!("waitKey()") };
        //if cpu.state.pc() >= 0x0909 { println!("_enable_timer0") };
        //if (cpu.state.pc() >= 0x0da3 && cpu.state.pc() <= 0xda3+0x200) { // trace in _main
        //if (cpu.state.pc() >= 0x0c80 && cpu.state.pc() <= 0xc80+0x80) { // trace in _wait_ESP32
        //if (cpu.state.pc() >= 0x0909 && cpu.state.pc() <= 0x909+0x80) { // trace in _enable_timer0
        //if (cpu.state.pc() >= 0x0860 && cpu.state.pc() <= 0x8c0) { // trace in _enable_timer0
        //if (cpu.state.pc() >= 0x2b7f && cpu.state.pc() <= 0x2b7f+0x126) { // trace in _init_timer0
        //if cpu.state.pc() >= 0xb7d2 && cpu.state.pc() <= 0xb7d2+0x100 { // trace in __print_sendstring
        //if true {
        if false {
           cpu.set_trace(true);
        }
        cpu.execute_instruction(&mut machine);
        cpu.set_trace(false);
    }
}
        //if (cpu.state.pc() >= 0xb983 && cpu.state.pc() <= 0xb983+0x100) { // trace in __lmulu
        //if (cpu.state.pc() >= 0xb0dc && cpu.state.pc() <= 0xb0dc+0x1000) { // trace in _init_timer0
