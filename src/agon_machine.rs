use crate::Machine;
use crate::Environment;
use crate::Cpu;
use crate::registers::*;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::collections::HashMap;
use std::io::{ Seek, SeekFrom, Read, Write };

const ROM_SIZE: usize = 0x40000; // 256 KiB
const RAM_SIZE: usize = 0x80000; // 512 KiB
const MEM_SIZE: usize = ROM_SIZE + RAM_SIZE;

mod mos {
    // FatFS struct FIL
    pub const SIZEOF_MOS_FIL_STRUCT: u32 = 36;
    pub const FIL_MEMBER_OBJSIZE: u32 = 11;
    pub const FIL_MEMBER_FPTR: u32 = 17;
    // f_open mode (3rd arg)
    //pub const FA_READ: u32 = 1;
    pub const FA_WRITE: u32 = 2;
    pub const FA_CREATE_NEW: u32 = 4;
}

pub struct AgonMachine {
    mem: [u8; MEM_SIZE],
    tx: Sender<u8>,
    rx: Receiver<u8>,
    rx_buf: Option<u8>,
    // map from MOS fatfs FIL struct ptr to rust File handle
    open_files: HashMap<u32, std::fs::File>,
    enable_hostfs: bool,
    vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
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
    pub fn new(tx : Sender<u8>, rx : Receiver<u8>, vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>) -> AgonMachine {
        AgonMachine {
            mem: [0; MEM_SIZE],
            tx,
            rx,
            rx_buf: None,
            open_files: HashMap::new(),
            enable_hostfs: true,
            vsync_counter
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

        // checksum the loaded MOS, to identify supported versions
        let checksum = z80_mem_tools::checksum(self, 0, code.len() as u32);
        if checksum != 0xc102d8 {
            println!("WARNING: Unsupported MOS version (only 1.03 is supported): disabling hostfs");
            self.enable_hostfs = false;
        }
    }

    fn hostfs_mos_f_close(&mut self, cpu: &mut Cpu) {
        let mut env = Environment::new(&mut cpu.state, self);
        let fptr = env.peek24(env.state.sp() + 3);
        env.state.reg.set24(Reg16::HL, 0); // ok
        env.subroutine_return();

        // closes in Drop
        self.open_files.remove(&fptr);
    }

    fn hostfs_mos_f_gets(&mut self, cpu: &mut Cpu) {
        let mut buf = self._peek24(cpu.state.sp() + 3);
        let max_len = self._peek24(cpu.state.sp() + 6);
        let fptr = self._peek24(cpu.state.sp() + 9);

        match self.open_files.get(&fptr) {
            Some(mut f) => {
                let mut line = vec![];
                let mut host_buf = vec![0; 1];
                for _ in 0..max_len {
                    f.read(host_buf.as_mut_slice()).unwrap();
                    line.push(host_buf[0]);

                    if host_buf[0] == 10 || host_buf[0] == 0 { break; }
                }
                // no f.tell()...
                let fpos = f.seek(SeekFrom::Current(0)).unwrap();
                // save file position to FIL.fptr
                self._poke24(fptr + mos::FIL_MEMBER_FPTR, fpos as u32);
                for b in line {
                    self.poke(buf, b);
                    buf += 1;
                }
                self.poke(buf, 0);
                cpu.state.reg.set24(Reg16::HL, 0); // success
            }
            None => {
                cpu.state.reg.set24(Reg16::HL, 1); // error
            }
        }
        let mut env = Environment::new(&mut cpu.state, self);
        env.subroutine_return();
    }

    fn hostfs_mos_f_write(&mut self, cpu: &mut Cpu) {
        let fptr = self._peek24(cpu.state.sp() + 3);
        let buf = self._peek24(cpu.state.sp() + 6);
        let num = self._peek24(cpu.state.sp() + 9);
        let num_written_ptr = self._peek24(cpu.state.sp() + 9);
        //println!("f_write(${:x}, ${:x}, {}, ${:x})", fptr, buf, num, num_written_ptr);

        match self.open_files.get(&fptr) {
            Some(mut f) => {
                for i in 0..num {
                    let byte = self.peek(buf + i);
                    f.write(&[byte]).unwrap();
                }

                // no f.tell()...
                let fpos = f.seek(SeekFrom::Current(0)).unwrap();
                // save file position to FIL.fptr
                self._poke24(fptr + mos::FIL_MEMBER_FPTR, fpos as u32);

                // inform caller that all bytes were written
                self._poke24(num_written_ptr, num);

                // success
                cpu.state.reg.set24(Reg16::HL, 0);
            }
            None => {
                // error
                cpu.state.reg.set24(Reg16::HL, 1);
            }
        }

        let mut env = Environment::new(&mut cpu.state, self);
        env.subroutine_return();
    }

    fn hostfs_mos_f_read(&mut self, cpu: &mut Cpu) {
        //fr = f_read(&fil, (void *)address, fSize, &br);		
        let fptr = self._peek24(cpu.state.sp() + 3);
        let mut buf = self._peek24(cpu.state.sp() + 6);
        let len = self._peek24(cpu.state.sp() + 9);
        match self.open_files.get(&fptr) {
            Some(mut f) => {
                let mut host_buf: Vec<u8> = vec![0; len as usize];
                f.read(host_buf.as_mut_slice()).unwrap();
                // no f.tell()...
                let fpos = f.seek(SeekFrom::Current(0)).unwrap();
                // copy to agon ram 
                for b in host_buf {
                    self.poke(buf, b);
                    buf += 1;
                }
                // save file position to FIL.fptr
                self._poke24(fptr + mos::FIL_MEMBER_FPTR, fpos as u32);

                cpu.state.reg.set24(Reg16::HL, 0); // ok
            }
            None => {
                cpu.state.reg.set24(Reg16::HL, 1); // error
            }
        }
        let mut env = Environment::new(&mut cpu.state, self);
        env.subroutine_return();
    }

    fn hostfs_mos_f_open(&mut self, cpu: &mut Cpu) {
        let fptr = self._peek24(cpu.state.sp() + 3);
        let filename = {
            let ptr = self._peek24(cpu.state.sp() + 6);
            // MOS filenames may not be valid utf-8
            unsafe {
                String::from_utf8_unchecked(z80_mem_tools::get_cstring(self, ptr))
            }
        };
        let mode = self._peek24(cpu.state.sp() + 9);
        //println!("f_open(${:x}, \"{}\", {})", fptr, filename.trim_end(), mode);
        match std::fs::File::options()
            .read(true)
            .write(mode & mos::FA_WRITE != 0)
            .create(mode & mos::FA_CREATE_NEW != 0)
            .open(filename.trim_end()) {
            Ok(mut f) => {
                // wipe the FIL structure
                z80_mem_tools::memset(self, fptr, 0, mos::SIZEOF_MOS_FIL_STRUCT);

                // save the size in the FIL structure
                let mut file_len = f.seek(SeekFrom::End(0)).unwrap();
                f.seek(SeekFrom::Start(0)).unwrap();

                // XXX don't support files larger than 512KiB
                file_len = file_len.min(1<<19);

                // store file len in fatfs FIL structure
                self._poke24(fptr + mos::FIL_MEMBER_OBJSIZE, file_len as u32);
                
                // store mapping from MOS *FIL to rust File
                self.open_files.insert(fptr, f);

                cpu.state.reg.set24(Reg16::HL, 0); // ok
            }
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => cpu.state.reg.set24(Reg16::HL, 4),
                    _ => cpu.state.reg.set24(Reg16::HL, 1)
                }
            }

        }
        let mut env = Environment::new(&mut cpu.state, self);
        env.subroutine_return();
    }

    pub fn start(&mut self) {
        let mut cpu = Cpu::new_ez80();
        let mut last_vsync_count = 0_u32;

        self.load_mos();

        cpu.state.set_pc(0x0000);

        loop {
            // fire uart interrupt
            if cpu.state.instructions_executed % 1024 == 0 && self.maybe_fill_rx_buf() != None {
                let mut env = Environment::new(&mut cpu.state, self);
                env.interrupt(0x18); // uart0_handler
            }

            // fire vsync interrupt
            {
                let cur_vsync_count = self.vsync_counter.load(std::sync::atomic::Ordering::Relaxed);
                if cur_vsync_count != last_vsync_count {
                    last_vsync_count = cur_vsync_count;
                    let mut env = Environment::new(&mut cpu.state, self);
                    env.interrupt(0x32);
                }
            }

            if self.enable_hostfs {
                if cpu.state.pc() == 0x822b { self.hostfs_mos_f_close(&mut cpu); }
                if cpu.state.pc() == 0x9c91 { self.hostfs_mos_f_gets(&mut cpu); }
                if cpu.state.pc() == 0x785e { self.hostfs_mos_f_read(&mut cpu); }
                if cpu.state.pc() == 0x738c { self.hostfs_mos_f_open(&mut cpu); }
                if cpu.state.pc() == 0x7c10 { self.hostfs_mos_f_write(&mut cpu); }
            }

            cpu.execute_instruction(self);
        }
    }
}

// misc Machine tools
mod z80_mem_tools {
    use crate::Machine;

    pub fn memset<M: Machine>(machine: &mut M, address: u32, fill: u8, count: u32) {
        for loc in address..(address + count) {
            machine.poke(loc, fill);
        }
    }

    pub fn get_cstring<M: Machine>(machine: &M, address: u32) -> Vec<u8> {
        let mut s: Vec<u8> = vec![];
        let mut ptr = address;

        loop {
            match machine.peek(ptr) {
                0 => break,
                b => s.push(b)
            }
            ptr += 1;
        }
        s
    }

    pub fn checksum<M: Machine>(machine: &M, start: u32, len: u32) -> u32 {
        let mut checksum = 0u32;
        for i in (start..(start+len)).step_by(3) {
            checksum ^= machine._peek24(i as u32);
        }
        checksum
    }
}
