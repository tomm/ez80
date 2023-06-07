// misc Machine tools
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
