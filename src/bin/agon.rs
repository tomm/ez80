use iz80::AgonMachine;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::io::{ self, BufRead, Write };

fn send_bytes(tx: &Sender<u8>, msg: &Vec<u8>) {
    for b in msg {
        tx.send(*b).unwrap();
    }
}

fn send_keys(tx: &Sender<u8>, msg: &str) {
    for key in msg.as_bytes() {
        // cmd, len, keycode, modifiers, vkey, keydown
        // key down
        tx.send(0x81).unwrap();
        tx.send(4).unwrap();
        tx.send(*key).unwrap();
        tx.send(0).unwrap();
        tx.send(0).unwrap();
        tx.send(1).unwrap();

        // key up
        tx.send(0x81).unwrap();
        tx.send(4).unwrap();
        tx.send(*key).unwrap();
        tx.send(0).unwrap();
        tx.send(0).unwrap();
        tx.send(0).unwrap();
    }
}

// Fake VDP. Minimal for MOS to work, outputting to stdout */
fn handle_vdp(tx_to_ez80: &Sender<u8>, rx_from_ez80: &Receiver<u8>) -> bool {
    match rx_from_ez80.try_recv() {
        Ok(data) => {
            match data {
                // one zero byte sent before everything else. real VDP ignores
                0 => {},
                1 => {},
                0xa => println!(),
                0xd => {},
                v if v >= 0x20 && v != 0x7f => {
                    print!("{}", char::from_u32(data as u32).unwrap());
                }
                // VDP system control
                0x17 => {
                    match rx_from_ez80.recv().unwrap() {
                        // video
                        0 => {
                            match rx_from_ez80.recv().unwrap() {
                                // general poll. echo back the sent byte
                                0x80 => {
                                    let resp = rx_from_ez80.recv().unwrap();
                                    send_bytes(&tx_to_ez80, &vec![0x80, 1, resp]);
                                }
                                // video mode info
                                0x86 => {
                                    let w: u16 = 640;
                                    let h: u16 = 400;
                                    send_bytes(&tx_to_ez80, &vec![
                                       0x86, 7,
                                       (w & 0xff) as u8, ((w>>8) & 0xff) as u8,
                                       (h & 0xff) as u8, ((h>>8) & 0xff) as u8, 80, 25, 1
                                    ]);
                                }
                                v => {
                                    println!("unknown packet VDU 0x17, 0, 0x{:x}", v);

                                }
                            }
                        }
                        v => {
                            println!("unknown packet VDU 0x17, 0x{:x}", v);
                        }
                    }
                }
                _ => {
                    println!("Unknown packet VDU 0x{:x}", data);//char::from_u32(data as u32).unwrap());
                }
            }
            std::io::stdout().flush().unwrap();
            true
        }
        Err(mpsc::TryRecvError::Disconnected) => panic!(),
        Err(mpsc::TryRecvError::Empty) => false
    }
}

fn start_vdp(tx_vdp_to_ez80: Sender<u8>, rx_ez80_to_vdp: Receiver<u8>,
             vsync_counter_vdp: std::sync::Arc<std::sync::atomic::AtomicU32>) {
    let (tx_stdin, rx_stdin): (Sender<String>, Receiver<String>) = mpsc::channel();
    let mut start_time = Some(std::time::SystemTime::now());
    let mut last_vsync = std::time::SystemTime::now();

    // to avoid blocking on stdin, use a thread and channel to read from it
    let _stdin_thread = std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            tx_stdin.send(line.unwrap()).unwrap();
        }
    });

    println!("Tom\'s Fake VDP Version 1.03");

    loop {
        if !handle_vdp(&tx_vdp_to_ez80, &rx_ez80_to_vdp) {
            // no packets from ez80. sleep a little
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let now = std::time::SystemTime::now();

        // a fake vsync every 16ms
        if now.duration_since(last_vsync).unwrap() >=
            std::time::Duration::from_millis(16) {
            // notify ez80 by incrementing a shared atomic integer
            vsync_counter_vdp.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            last_vsync = now;
        }

        // emit stdin input as keyboard events to the ez80, line by line
        if let Some(t) = start_time {
            let elapsed = now.duration_since(t).unwrap();
            if elapsed > std::time::Duration::from_secs(2) {
                match rx_stdin.try_recv() {
                    Ok(line) => {
                        send_keys(&tx_vdp_to_ez80, &line);
                        send_keys(&tx_vdp_to_ez80, "\r");
                        start_time = Some(std::time::SystemTime::now());
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {},
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
        }
    }
}

fn main() {
    let (tx_vdp_to_ez80, rx_vdp_to_ez80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_ez80_to_vdp, rx_ez80_to_vdp): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let vsync_counter_vdp = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let vsync_counter_ez80 = vsync_counter_vdp.clone();

    let _cpu_thread = std::thread::spawn(move || {
        let mut machine = AgonMachine::new(tx_ez80_to_vdp, rx_vdp_to_ez80, vsync_counter_ez80);
        machine.start();
    });

    start_vdp(tx_vdp_to_ez80, rx_ez80_to_vdp, vsync_counter_vdp);
}
