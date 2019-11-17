mod instructions;

use std::env;
use std::fs::File;
use std::io::prelude::*;

use instructions::instructions;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let filename = &args[1];
    let mut f = File::open(filename).expect("file not found");

    let mut data = Vec::new();
    f.read_to_end(&mut data).expect("error reading file");
    println!("read {} bytes.", data.len());

    let mut pos = 0usize;
    while pos < data.len() {
        let mut instr = &instructions[data[pos] as usize];
        if instr.operation == instructions::Operation::PREFIX {
            instr = &instructions[data[pos+1] as usize];
        }
        let instr = instr;

        match instr.length {
            2 => println!("0x{:04x}:   {:02x}{:02x}    {} 0x{:02x}",
                        pos, data[pos], data[pos+1], instr.mnemo, data[pos+1]),
            3 => println!("0x{:04x}:   {:02x}{:02x}{:02x}  {} 0x{:02x}{:02x}",
                        pos, data[pos], data[pos+1], data[pos+2], instr.mnemo, data[pos+2], data[pos+1]),
            _ => println!("0x{:04x}:   {:02x}      {}",
                        pos, data[pos], instr.mnemo),
        }
        pos += instr.length as usize;
    }
}
