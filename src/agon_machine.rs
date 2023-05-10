use crate::Machine;
use crate::Environment;
use crate::Cpu;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

const ROM_SIZE: usize = 0x40000; // 256 KiB
const RAM_SIZE: usize = 0x80000; // 512 KiB
const MEM_SIZE: usize = ROM_SIZE + RAM_SIZE;

pub struct AgonMachine {
    mem: [u8; MEM_SIZE],
    tx: Sender<u8>,
    rx: Receiver<u8>,
    rx_buf: Option<u8>
}

impl Machine for AgonMachine {
    fn peek(&self, address: u32) -> u8 {
        self.mem[address as usize]
    }

    fn poke(&mut self, address: u32, value: u8) {
        self.mem[address as usize] = value;
    }

    fn port_in(&mut self, address: u16) -> u8 {
        //println!("IN({:02X})", address);
        if address == 0xa2 {
            0x0 // UART0 clear to send
        } else if address == 0xc0 {
            // uart0 receive
            self.maybe_fill_rx_buf();

            let maybe_data = self.rx_buf;
            self.rx_buf = None;

            match maybe_data {
                Some(data) => data,
                None => 0
            }
        } else if address == 0xc5 {
            self.maybe_fill_rx_buf();

            match self.rx_buf {
                Some(_) => 0x41,
                None => 0x40
            }
            // UART_LSR_ETX		EQU 	%40 ; Transmit empty (can send)
            // UART_LSR_RDY		EQU	%01		; Data ready (can receive)
        } else if address == 0x81 /* timer0 low byte */ {
            0x0
        } else if address == 0x82 /* timer0 high byte */ {
            0x0
        } else {
            0
        }
    }
    fn port_out(&mut self, address: u16, value: u8) {
        //println!("OUT(${:02X}) = ${:x}", address, value);
        if address == 0xc0 /* UART0_REG_THR */ {
            /* Send data to VDP */
            self.tx.send(value).unwrap();

            //print!("{}", char::from_u32(value as u32).unwrap());
            //std::io::stdout().flush().unwrap();
        }
    }
}

impl AgonMachine {
    pub fn new(tx : Sender<u8>, rx : Receiver<u8>) -> AgonMachine {
        AgonMachine {
            mem: [0; MEM_SIZE],
            tx,
            rx,
            rx_buf: None
        }
    }

    fn maybe_fill_rx_buf(&mut self) -> Option<u8> {
        if self.rx_buf == None {
            self.rx_buf = match self.rx.try_recv() {
                Ok(data) => Some(data),
                Err(mpsc::TryRecvError::Disconnected) => panic!(),
                Err(mpsc::TryRecvError::Empty) => None
            }
        }
        self.rx_buf
    }

    fn load_mos(&mut self) {
        let code = match std::fs::read("MOS.bin") {
            Ok(data) => data,
            Err(e) => {
                println!("Error opening MOS.bin: {:?}", e);
                std::process::exit(-1);
            }
        };
        for (i, e) in code.iter().enumerate() {
            self.poke(i as u32, *e);
        }
    }

    pub fn start(&mut self) {
        let mut cpu = Cpu::new_ez80();
        self.load_mos();
        // Run emulation
        cpu.state.set_pc(0x0000);
        //for _ in 0..3600 {
        loop {
            //machine.poke(0xBC504, 1); // _gp variable
            //if cpu.state.pc() == 0x0 { println!("_reset()") };
            /*
            if cpu.state.pc() == 0x0d96 { println!("main()") };
            if cpu.state.pc() == 0x0d68 { println!("init_interrupts()") };
            if cpu.state.pc() == 0x0847 { println!("init_rtc()") };
            if cpu.state.pc() == 0x349d { println!("init_spi()") };
            if cpu.state.pc() == 0xa92a { println!("init_UART0()") };
            if cpu.state.pc() == 0xa948 { println!("init_UART1()") };
            if cpu.state.pc() == 0x0c80 { println!("wait_ESP32()") };
            if cpu.state.pc() == 0xa966 { println!("open_UART0()") };
            if cpu.state.pc() == 0x2b7f { println!("init_timer0()") };
            if cpu.state.pc() == 0x2c36 { println!("enable_timer0()") };
            if cpu.state.pc() == 0xae16 { println!("__print_sendstring()") };
            if cpu.state.pc() == 0x0909 {
                println!("putch({})", machine.peek(cpu.state.reg.get24(Reg16::SP)+3));
            };
            if cpu.state.pc() == 0x08d4 {
                println!("UART0_serial_PUTCH({})", cpu.state.reg.get8(Reg8::A));
            };
            if cpu.state.pc() == 0x084e { println!("UART0_wait_CTS()") };
            if cpu.state.pc() == 0x0860 { println!("UART0_serial_TX()") };
            if cpu.state.pc() == 0x063a { println!("wait_timer0()") };
            if cpu.state.pc() == 0xb162 { println!("uitoa()") };
            if cpu.state.pc() == 0xb7b5 { println!("itol()") };
            if cpu.state.pc() == 0xb09c { println!("strlen()") };
            if cpu.state.pc() == 0xb184 { println!("ultoa()") };
            if cpu.state.pc() == 0xb418 { println!("lcmpzero()") };
            if cpu.state.pc() == 0xb6e0 { println!("_u_reverse()") };
            if cpu.state.pc() == 0xb7bc { println!("lcmpu()") };
            if cpu.state.pc() == 0xadfe { println!("setflag()") };
            if cpu.state.pc() == 0xb7d2 { println!("case8D()") };
            if cpu.state.pc() == 0x2c92 { println!("wait_VDP()") };
            if cpu.state.pc() == 0xb8bd { println!("_lremu()") };
            if cpu.state.pc() == 0xb1e3 { println!("_ldvrmu()") };
            if cpu.state.pc() == 0xb12c { println!("_ldivu()") };
            if cpu.state.pc() == 0xb528 { println!("_print_send()") };
            if cpu.state.pc() == 0xb8cd { println!("_print_putch()") };
            if cpu.state.pc() == 0xb815 { println!("_indcall()") };
            if cpu.state.pc() == 0xb024 { println!("_print_uputch()") };
            if cpu.state.pc() == 0x72f7 { println!("_f_mount()") };
            */
            //if cpu.state.pc() == 0x72f7 { println!("_f_mount()") };
            //if cpu.state.pc() == 0x923 { println!("getch()") };
            if cpu.state.instructions_executed % 10000 == 0 {
                let mut env = Environment::new(&mut cpu.state, self);
                env.interrupt(0x18); // uart0_handler
            }
            //if cpu.state.pc() == 0x2657 {
                //println!("waitKey()");
            //};
            //if cpu.state.pc() >= 0x0909 { println!("_enable_timer0") };
            //if (cpu.state.pc() >= 0x0da3 && cpu.state.pc() <= 0xda3+0x200) { // trace in _main
            //if (cpu.state.pc() >= 0x0c80 && cpu.state.pc() <= 0xc80+0x80) { // trace in _wait_ESP32
            //if (cpu.state.pc() >= 0x0909 && cpu.state.pc() <= 0x909+0x80) { // trace in _enable_timer0
            //if (cpu.state.pc() >= 0x0860 && cpu.state.pc() <= 0x8c0) { // trace in _enable_timer0
            //if (cpu.state.pc() >= 0x2b7f && cpu.state.pc() <= 0x2b7f+0x126) { // trace in _init_timer0
            //if cpu.state.pc() >= 0x833 && cpu.state.pc() <= 0x833+0x40 { // trace in _set_vector
            //if true {
            if false {
               cpu.set_trace(true);
            }
            cpu.execute_instruction(self);
            cpu.set_trace(false);
        }
    }
}
