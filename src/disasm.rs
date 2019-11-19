mod instructions;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashSet;

use instructions::instructions;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let filename = &args[1];
    let mut f = File::open(filename).expect("file not found");

    let mut data = Vec::new();
    f.read_to_end(&mut data).expect("error reading file");
    println!("read {} bytes.", data.len());

    let mut targets = HashSet::new();

    let mut pos = 0usize;
    while pos < data.len() {
        let mut instr = &instructions[data[pos] as usize];
        if instr.operation == instructions::Operation::PREFIX {
            instr = &instructions[data[pos+1] as usize + 0x100];
        }
        let instr = instr;
        match instr.operation {
            instructions::Operation::JUMP {..} => {
                let target = if instr.length == 3 {
                    data[pos+1] as usize | (data[pos+2] as usize) << 8
                }
                else {
                    (data[pos+1] as i8 as isize + pos as isize + 2) as usize
                };
                targets.insert(target);
            },
            _ => (),
        }
        pos += instr.length as usize;
    }

    pos = 0usize;
    while pos < data.len() {
        if targets.contains(&pos) {
            println!("\naddr_0x{:04x}:", pos);
        }
        let mut instr = &instructions[data[pos] as usize];
        if instr.operation == instructions::Operation::PREFIX {
            instr = &instructions[data[pos+1] as usize + 0x100];
        }
        let instr = instr;

        let mut comment = String::new();

        match instr.operation {
            instructions::Operation::JUMP {..} => {
                let target = if instr.length == 3 {
                    data[pos+1] as usize | (data[pos+2] as usize) << 8
                }
                else {
                    (data[pos+1] as i8 as isize + pos as isize + 2) as usize
                };
                comment = format!("  // addr_0x{:04x}", target);
            },
            _ => (),
        }

        match instr.length {
            2 => println!("0x{:04x}:   {:02x}{:02x}    {} 0x{:02x}{}",
                        pos, data[pos], data[pos+1], instr.mnemo, data[pos+1], comment),
            3 => println!("0x{:04x}:   {:02x}{:02x}{:02x}  {} 0x{:02x}{:02x}{}",
                        pos, data[pos], data[pos+1], data[pos+2], instr.mnemo, data[pos+2], data[pos+1], comment),
            _ => println!("0x{:04x}:   {:02x}      {}",
                        pos, data[pos], instr.mnemo),
        }
        pos += instr.length as usize;
    }
}
