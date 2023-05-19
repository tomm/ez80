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
    // FatFS struct FILINFO
    pub const SIZEOF_MOS_FILINFO_STRUCT: u32 = 278;
	pub const FILINFO_MEMBER_FSIZE_U32: u32 = 0;
    //pub const FILINFO_MEMBER_FDATE_U16: u32 = 4;
    //pub const FILINFO_MEMBER_FTIME_U16: u32 = 6;
    pub const FILINFO_MEMBER_FATTRIB_U8: u32 = 8;
    //pub const FILINFO_MEMBER_ALTNAME_13BYTES: u32 = 9;
    pub const FILINFO_MEMBER_FNAME_256BYTES: u32 = 22;
    // f_open mode (3rd arg)
    //pub const FA_READ: u32 = 1;
    pub const FA_WRITE: u32 = 2;
    pub const FA_CREATE_NEW: u32 = 4;
}

struct MosMap {
    pub f_chdir: u32,
    pub f_chdrive: u32,
    pub f_close: u32,
    pub f_closedir: u32,
    pub f_getcwd: u32,
    pub f_getfree: u32,
    pub f_getlabel: u32,
    pub f_gets: u32,
    pub f_lseek: u32,
    pub f_mkdir: u32,
    pub f_mount: u32,
    pub f_open: u32,
    pub f_opendir: u32,
    pub f_printf: u32,
    pub f_putc: u32,
    pub f_puts: u32,
    pub f_read: u32,
    pub f_readdir: u32,
    pub f_rename: u32,
    pub f_setlabel: u32,
    pub f_stat: u32,
    pub f_sync: u32,
    pub f_truncate: u32,
    pub f_unlink: u32,
    pub f_write: u32,
}

static MOS_103_MAP: MosMap = MosMap {
    f_chdir    : 0x82B2,
    f_chdrive  : 0x827A,
    f_close    : 0x822B,
    f_closedir : 0x8B5B,
    f_getcwd   : 0x8371,
    f_getfree  : 0x8CE8,
    f_getlabel : 0x9816,
    f_gets     : 0x9C91,
    f_lseek    : 0x8610,
    f_mkdir    : 0x92F6,
    f_mount    : 0x72F7,
    f_open     : 0x738C,
    f_opendir  : 0x8A52,
    f_printf   : 0x9F11,
    f_putc     : 0x9E8E,
    f_puts     : 0x9EC4,
    f_read     : 0x785E,
    f_readdir  : 0x8B92,
    f_rename   : 0x9561,
    f_setlabel : 0x99DB,
    f_stat     : 0x8C55,
    f_sync     : 0x8115,
    f_truncate : 0x8F78,
    f_unlink   : 0x911A,
    f_write    : 0x7C10,
};

pub struct AgonMachine {
    mem: [u8; MEM_SIZE],
    tx: Sender<u8>,
    rx: Receiver<u8>,
    rx_buf: Option<u8>,
    // map from MOS fatfs FIL struct ptr to rust File handle
    open_files: HashMap<u32, std::fs::File>,
    open_dirs: HashMap<u32, std::fs::ReadDir>,
    enable_hostfs: bool,
    hostfs_current_dir: std::path::PathBuf,
    vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl Machine for AgonMachine {
    fn peek(&self, address: u32) -> u8 {
        if address >= 0xc0000 {
            println!("eZ80 memory read out of bounds: ${:x}", address);
            0 
        } else {
            self.mem[address as usize]
        }
    }

    fn poke(&mut self, address: u32, value: u8) {
        if address >= 0xc0000 || address < 0x40000 {
            println!("eZ80 memory write out of bounds: ${:x}", address);
        } else {
            self.mem[address as usize] = value;
        }
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
            open_dirs: HashMap::new(),
            enable_hostfs: true,
            hostfs_current_dir: std::path::PathBuf::new(),
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
            self.mem[i] = *e;
        }

        // checksum the loaded MOS, to identify supported versions
        let checksum = z80_mem_tools::checksum(self, 0, code.len() as u32);
        if checksum != 0xc102d8 {
            println!("WARNING: Unsupported MOS version (only 1.03 is supported): disabling hostfs");
            self.enable_hostfs = false;
        }
    }

    fn hostfs_mos_f_getlabel(&mut self, cpu: &mut Cpu) {
        let mut buf = self._peek24(cpu.state.sp() + 6);
        let label = "hostfs";
        for b in label.bytes() {
            self.poke(buf, b);
            buf += 1;
        }
        self.poke(buf, 0);

        // success
        cpu.state.reg.set24(Reg16::HL, 0); // success

        let mut env = Environment::new(&mut cpu.state, self);
        env.subroutine_return();
    }

    fn hostfs_mos_f_close(&mut self, cpu: &mut Cpu) {
        let fptr = self._peek24(cpu.state.sp() + 3);
        //println!("f_close(${:x})", fptr);

        // closes on Drop
        self.open_files.remove(&fptr);

        // success
        cpu.state.reg.set24(Reg16::HL, 0);

        Environment::new(&mut cpu.state, self).subroutine_return();
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
                // save file position to FIL.fptr U32
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

    fn hostfs_mos_f_closedir(&mut self, cpu: &mut Cpu) {
        let dir_ptr = self._peek24(cpu.state.sp() + 3);
        // closes on Drop
        self.open_dirs.remove(&dir_ptr);

        // success
        cpu.state.reg.set24(Reg16::HL, 0); // success

        let mut env = Environment::new(&mut cpu.state, self);
        env.subroutine_return();
    }

    fn hostfs_mos_f_readdir(&mut self, cpu: &mut Cpu) {
        let dir_ptr = self._peek24(cpu.state.sp() + 3);
        let file_info_ptr = self._peek24(cpu.state.sp() + 6);

        // clear the FILINFO struct
        z80_mem_tools::memset(self, file_info_ptr, 0, mos::SIZEOF_MOS_FILINFO_STRUCT);

        match self.open_dirs.get_mut(&dir_ptr) {
            Some(dir) => {

                match dir.next() {
                    Some(Ok(dir_entry)) => {
                        let path = dir_entry.path();
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            // XXX to_str can fail if not utf-8
                            // write file name
                            z80_mem_tools::memcpy_to_z80(
                                self, file_info_ptr + mos::FILINFO_MEMBER_FNAME_256BYTES,
                                path.file_name().unwrap().to_str().unwrap().as_bytes()
                            );

                            // write file length (U32)
                            self._poke24(file_info_ptr + mos::FILINFO_MEMBER_FSIZE_U32, metadata.len() as u32);
                            self.poke(file_info_ptr + mos::FILINFO_MEMBER_FSIZE_U32 + 3, (metadata.len() >> 24) as u8);

                            // is directory?
                            if metadata.is_dir() {
                                self.poke(file_info_ptr + mos::FILINFO_MEMBER_FATTRIB_U8, 0x10 /* AM_DIR */);
                            }

                            // TODO set fdate, ftime

                            // success
                            cpu.state.reg.set24(Reg16::HL, 0);
                        } else {
                            // hm. why might std::fs::metadata fail?
                            z80_mem_tools::memcpy_to_z80(
                                self, file_info_ptr + mos::FILINFO_MEMBER_FNAME_256BYTES,
                                "<error reading file metadata>".as_bytes()
                            );
                            cpu.state.reg.set24(Reg16::HL, 0);
                        }
                    }
                    Some(Err(_)) => {
                        cpu.state.reg.set24(Reg16::HL, 1); // error
                    }
                    None => {
                        // directory has been read to the end.
                        // do nothing, since FILINFO.fname[0] == 0 indicates to MOS that
                        // the directory end has been reached

                        // success
                        cpu.state.reg.set24(Reg16::HL, 0);
                    }
                }
            }
            None => {
                cpu.state.reg.set24(Reg16::HL, 1); // error
            }
        }

        Environment::new(&mut cpu.state, self).subroutine_return();
    }

    fn hostfs_mos_f_chdir(&mut self, cpu: &mut Cpu) {
        let cd_to_ptr = self._peek24(cpu.state.sp() + 3);
        let cd_to = unsafe {
            // MOS filenames may not be valid utf-8
            String::from_utf8_unchecked(z80_mem_tools::get_cstring(self, cd_to_ptr))
        };
        //println!("f_chdir({})", cd_to);

        let mut new_path = self.hostfs_current_dir.clone();
        if cd_to == ".." {
            new_path.pop();
        } else {
            new_path = new_path.join(cd_to);
        }

        match std::fs::metadata(std::env::current_dir().unwrap().join(&new_path)) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    //println!("setting path to {:?}", &new_path);
                    self.hostfs_current_dir = new_path;
                    cpu.state.reg.set24(Reg16::HL, 0);
                } else {
                    cpu.state.reg.set24(Reg16::HL, 1);
                }
            }
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        cpu.state.reg.set24(Reg16::HL, 4);
                    }
                    _ => {
                        cpu.state.reg.set24(Reg16::HL, 1);
                    }
                }
            }
        }
        Environment::new(&mut cpu.state, self).subroutine_return();
    }

    fn hostfs_mos_f_mount(&mut self, cpu: &mut Cpu) {
        // always success. hostfs is mounted
        cpu.state.reg.set24(Reg16::HL, 0); // ok
        Environment::new(&mut cpu.state, self).subroutine_return();
    }

    fn hostfs_mos_f_opendir(&mut self, cpu: &mut Cpu) {
        //fr = f_opendir(&dir, path);
        let dir_ptr = self._peek24(cpu.state.sp() + 3);
        let path_ptr = self._peek24(cpu.state.sp() + 6);
        let path = unsafe {
            // MOS filenames may not be valid utf-8
            String::from_utf8_unchecked(z80_mem_tools::get_cstring(self, path_ptr))
        };
        //println!("f_opendir(${:x}, \"{}\")", dir_ptr, path.trim_end());

        match std::fs::read_dir(self.hostfs_path().join(path)) {
            Ok(dir) => {
                // XXX should clear the DIR struct in z80 ram
                
                // store in map of z80 DIR ptr to rust ReadDir
                self.open_dirs.insert(dir_ptr, dir);
                cpu.state.reg.set24(Reg16::HL, 0); // ok
            }
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        cpu.state.reg.set24(Reg16::HL, 4);
                    }
                    _ => {
                        cpu.state.reg.set24(Reg16::HL, 1);
                    }
                }
            }
        }

        cpu.state.reg.set24(Reg16::HL, 0); // ok
        let mut env = Environment::new(&mut cpu.state, self);
        env.subroutine_return();
    }

    fn hostfs_path(&mut self) -> std::path::PathBuf {
        std::env::current_dir().unwrap().join(&self.hostfs_current_dir)
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
        let path = match filename.chars().nth(0) {
            Some('/') => {
                std::env::current_dir().unwrap().join(filename.chars().skip(1).collect::<String>().trim_end())
            }
            _ => {
                self.hostfs_path().join(filename.trim_end())
            }
        };
        let mode = self._peek24(cpu.state.sp() + 9);
        //println!("f_open(${:x}, \"{}\", {})", fptr, &filename, mode);
        match std::fs::File::options()
            .read(true)
            .write(mode & mos::FA_WRITE != 0)
            .create(mode & mos::FA_CREATE_NEW != 0)
            .open(path) {
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

        let mut trace_for = 0;

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
                if cpu.state.pc() == MOS_103_MAP.f_close { self.hostfs_mos_f_close(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_gets { self.hostfs_mos_f_gets(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_read { self.hostfs_mos_f_read(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_open { self.hostfs_mos_f_open(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_write { self.hostfs_mos_f_write(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_chdir { self.hostfs_mos_f_chdir(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_chdrive { println!("Un-trapped fatfs call: f_chdrive"); }
                if cpu.state.pc() == MOS_103_MAP.f_closedir { self.hostfs_mos_f_closedir(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_getcwd { println!("Un-trapped fatfs call: f_getcwd"); }
                if cpu.state.pc() == MOS_103_MAP.f_getfree { println!("Un-trapped fatfs call: f_getfree"); }
                if cpu.state.pc() == MOS_103_MAP.f_getlabel { self.hostfs_mos_f_getlabel(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_lseek { println!("Un-trapped fatfs call: f_lseek"); }
                if cpu.state.pc() == MOS_103_MAP.f_mkdir { println!("Un-trapped fatfs call: f_mkdir"); }
                if cpu.state.pc() == MOS_103_MAP.f_mount { self.hostfs_mos_f_mount(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_opendir { self.hostfs_mos_f_opendir(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_printf { println!("Un-trapped fatfs call: f_printf"); }
                if cpu.state.pc() == MOS_103_MAP.f_putc { println!("Un-trapped fatfs call: f_putc"); }
                if cpu.state.pc() == MOS_103_MAP.f_puts { println!("Un-trapped fatfs call: f_puts"); }
                if cpu.state.pc() == MOS_103_MAP.f_readdir { self.hostfs_mos_f_readdir(&mut cpu); }
                if cpu.state.pc() == MOS_103_MAP.f_rename { println!("Un-trapped fatfs call: f_rename"); }
                if cpu.state.pc() == MOS_103_MAP.f_setlabel { println!("Un-trapped fatfs call: f_setlabel"); }
                if cpu.state.pc() == MOS_103_MAP.f_stat { println!("Un-trapped fatfs call: f_stat"); }
                if cpu.state.pc() == MOS_103_MAP.f_sync { println!("Un-trapped fatfs call: f_sync"); }
                if cpu.state.pc() == MOS_103_MAP.f_truncate { println!("Un-trapped fatfs call: f_truncate"); }
                if cpu.state.pc() == MOS_103_MAP.f_unlink { println!("Un-trapped fatfs call: f_unlink"); }
            }

            //if cpu.state.pc() == 0x43838 { trace_for = 1000; cpu.set_trace(true); }
            //if trace_for == 0 { cpu.set_trace(false); } else { trace_for -= 1; }

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

    pub fn memcpy_to_z80<M: Machine>(machine: &mut M, start: u32, data: &[u8]) {
        let mut loc = start;
        for byte in data {
            machine.poke(loc, *byte);
            loc += 1;
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
