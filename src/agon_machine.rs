use crate::Machine;
use crate::Environment;
use crate::Cpu;
use crate::registers::*;
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
            std::thread::sleep(std::time::Duration::from_millis(10));
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
            if cpu.state.instructions_executed % 1024 == 0 {
                let mut env = Environment::new(&mut cpu.state, self);
                env.interrupt(0x18); // uart0_handler
            }
            //if cpu.state.pc() >= 0xb06a && cpu.state.pc() <= 0xb06a+0x80 { // trace in toupper
            if false {
               cpu.set_trace(true);
            }
            cpu.execute_instruction(self);
            cpu.set_trace(false);
        }
    }
}
