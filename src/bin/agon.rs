use iz80::AgonMachine;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::io::Write;

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

fn main() {
    let (tx_vdp_to_ez80, rx_vdp_to_ez80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_ez80_to_vdp, rx_ez80_to_vdp): (Sender<u8>, Receiver<u8>) = mpsc::channel();

    let _cpu_thread = std::thread::spawn(move || {
        let mut machine = AgonMachine::new(tx_ez80_to_vdp, rx_vdp_to_ez80);
        machine.start();
    });

    let mut start_time = Some(std::time::SystemTime::now());
    let mut commands = vec!["run\r", "load helloworld.bin\r"];

    loop {
        if !handle_vdp(&tx_vdp_to_ez80, &rx_ez80_to_vdp) {
            // no packets from ez80. sleep a little
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if let Some(t) = start_time {
            let elapsed = std::time::SystemTime::now().duration_since(t).unwrap();
            if elapsed > std::time::Duration::from_secs(2) {
                start_time = Some(std::time::SystemTime::now());
                if let Some(cmd) = commands.pop() {
                    send_keys(&tx_vdp_to_ez80, cmd);
                }
            }
        }
    }
}
