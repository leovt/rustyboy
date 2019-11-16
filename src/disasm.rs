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
        let instr = &instructions[pos];
        pos += 1;
        println!("{}", instr.mnemo);
    }
}
