use iz80::AgonMachine;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::io::Write;

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

fn main() {
    let (tx_VDP2EZ80, rx_VDP2EZ80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_EZ802VDP, rx_EZ802VDP): (Sender<u8>, Receiver<u8>) = mpsc::channel();

    let cpu_thread = std::thread::spawn(move || {
        let mut machine = AgonMachine::new(tx_EZ802VDP, rx_VDP2EZ80);
        machine.start();
    });

    loop {
        // Fake VDP. Just echo messages from the ez80 to stdout :)
        let data = rx_EZ802VDP.recv().unwrap();
        print!("{}", char::from_u32(data as u32).unwrap());
        std::io::stdout().flush().unwrap();
        send_keys(&tx_VDP2EZ80, "Hello mos! ");
    }
}
