use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::collections::HashMap;

enum Argument {
    Reg8(u8),
    Reg8Ind(u8),
    Reg16(u8),
    Imm(u16),
    Addr16(u16),
    Addr8(u8),
    Addr8Ind(u8),
    Label(String),
}

use Argument::*;

fn parse_arg(arg: &str) -> Argument {
    match reg8(arg) { Some(r) => Reg8(r), None =>
    match reg8ind(arg) { Some(r) => Reg8Ind(r), None =>
    match reg16(arg) { Some(r) => Reg16(r), None =>
    match imm16(arg) { Some(d) => Imm(d), None =>
    match addr16(arg) { Some(a) => Addr16(a), None =>
    match addr8(arg) { Some(a) => Addr8(a), None =>
    match addr8ind(arg) { Some(a) => Addr8Ind(a), None =>
    Label(arg.to_string())
}}}}}}}}


fn reg8(name: &str) -> Option<u8> {
    match name {
        "B" => Some(0),
        "C" => Some(1),
        "D" => Some(2),
        "E" => Some(3),
        "H" => Some(4),
        "L" => Some(5),
        "(HL)" => Some(6),
        "A" => Some(7),
        _ => None
    }
}

fn reg8ind(name: &str) -> Option<u8> {
    match name {
        "(BC)"  => Some(0),
        "(DE)"  => Some(0x10),
        "(HL+)" => Some(0x20),
        "(HL-)" => Some(0x30),
        _ => None,
    }
}

fn reg16(name: &str) -> Option<u8> {
    match name {
        "BC" => Some(0),
        "DE" => Some(0x10),
        "HL" => Some(0x20),
        "SP" => Some(0x30),
        "AF" => Some(0x30),
        _ => None
    }
}

fn imm8(src: &str) -> Option<u8> {
    if src.starts_with("$") {
        u8::from_str_radix(src.split_at(1).1, 16).ok()
    }
    else if src.starts_with("0x") {
        u8::from_str_radix(src.split_at(2).1, 16).ok()
    }
    else if src.starts_with("0") {
        u8::from_str_radix(src, 8).ok()
    }
    else {
        u8::from_str_radix(src, 10).ok()
    }
}

fn imm16(src: &str) -> Option<u16> {
    if src.starts_with("$") {
        u16::from_str_radix(src.split_at(1).1, 16).ok()
    }
    else if src.starts_with("0x") {
        u16::from_str_radix(src.split_at(2).1, 16).ok()
    }
    else if src.starts_with("0") {
        u16::from_str_radix(src, 8).ok()
    }
    else {
        u16::from_str_radix(src, 10).ok()
    }
}

fn addr16(arg: &str) -> Option<u16> {
    if arg.starts_with("(") && arg.ends_with(")") {
        imm16(&arg[1..arg.len()-1])
    }
    else {None}
}

fn addr8(arg: &str) -> Option<u8> {
    let parts = arg.split("+").collect::<Vec<_>>();

    if parts.len()==2 &&
       parts[0].starts_with("(") &&
       parts[1].ends_with(")") &&
       imm16(&parts[0][1..]) == Some(0xff00) {
        imm8(&parts[1][..parts[1].len()-1])
    }
    else {None}
}

fn addr8ind(arg: &str) -> Option<u8> {
    let parts = arg.split("+").collect::<Vec<_>>();

    if parts.len()==2 &&
       parts[0].starts_with("(") &&
       parts[1].ends_with(")") &&
       imm16(&parts[0][1..]) == Some(0xff00) {
        reg8(&parts[1][..parts[1].len()-1])
    }
    else {None}
}

fn binop(base: u8, src:&str, data:&mut Vec<u8>) {
    match reg8(src) {
        Some(s) => data.push(base + s),
        None => {
            data.push(base + 0x46);
            data.push(imm8(src).expect("Could not parse imm8"))
        }
    }
}

fn ld(dst: &str, src:&str, data:&mut Vec<u8>, fixes:&mut Vec<(usize, String, bool)>) {
    match [parse_arg(dst), parse_arg(src)] {
        [Reg8(d), Reg8(s)] => data.push(0x40+d*0x08+s),
        [Reg8(d), Imm(i)] => {
            data.push(0x06+d*0x08);
            data.push(i as u8);
        },
        [Reg8(7), Reg8Ind(s)] => data.push(0x0a + s),
        [Reg8Ind(d), Reg8(7)] => data.push(0x02 + d),

        [Reg16(d), Imm(i)] => {
            data.push(0x01 + d);
            let [h,l] = i.to_be_bytes();
            data.push(l);
            data.push(h);
        },

        [Reg16(d), Label(label)] => {
            data.push(0x01 + d);
            data.push(0);
            data.push(0);
            fixes.push((data.len(), label, false));
        },

        [Reg8(7), Addr8(s)] => {data.push(0xf0); data.push(s)},
        [Addr8(d), Reg8(7)] => {data.push(0xe0); data.push(d)},

        [Reg8(7), Addr16(s)] => {
            data.push(0xfa);
            let [h,l] = s.to_be_bytes();
            data.push(l);
            data.push(h);
        }
        [Addr16(d), Reg8(7)] => {
            data.push(0xea);
            let [h,l] = d.to_be_bytes();
            data.push(l);
            data.push(h);
        }
        [Reg8(7), Addr8Ind(1)] => data.push(0xf2),
        [Addr8Ind(1), Reg8(7)] => data.push(0xe2),

        _ => panic!("can not parse LD {}, {}", dst, src)
    }
}

fn cond_offset(cond: &str) -> u8 {
    match cond {
        "NZ" => 0,
        "NC" => 0x10,
        "Z"  => 0x08,
        "C"  => 0x18,
        _ => panic!("illegal jump condition")
    }
}

fn jump(op: u8, addr: &str, relative:bool, data:&mut Vec<u8>, fixes:&mut Vec<(usize, String, bool)>)
{
    data.push(op);

    if !relative {
        match imm16(addr) {
            Some(tgt) => {
                let [h,l] = tgt.to_be_bytes();
                data.push(l);
                data.push(h);
            }
            _ => {
                data.push(0);
                data.push(0);
                fixes.push((data.len(), addr.to_string(), relative));
            }
        }
    }
    else {
        match imm8(addr) {
            Some(offset) => data.push(offset),
            None => {
                data.push(0);
                fixes.push((data.len(), addr.to_string(), relative));
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let filename_in = &args[1];
    let mut f = File::open(filename_in).expect("could not open file for reading");

    let mut data = Vec::new();
    let reader = BufReader::new(f);

    let mut fixes = Vec::new();
    let mut labels = HashMap::new();

    for line in reader.lines() {
        let line = line.expect("no line");
        let line = line.split(";").next().unwrap().trim();
        println!("{:?}", line);

        if line.ends_with(":") {
            let label = line.split(":").next().unwrap().trim().to_string();
            if labels.contains_key(&label) {
                panic!("label {} already defined", label);
            }
            labels.insert(label, data.len());
        }
        else {
            let components:Vec<&str> = line.split(|c| c==' ' || c==',').filter(|w| !w.is_empty()).collect();
            println!("{:?}", components);

            match components[..] {
                    [] => (),
                    ["LD", dst, src] => ld(dst, src, &mut data, &mut fixes),

                    ["ADD", src] => binop(0x80, src, &mut data),
                    ["ADC", src] => binop(0x88, src, &mut data),
                    ["SUB", src] => binop(0x90, src, &mut data),
                    ["SBC", src] => binop(0x98, src, &mut data),
                    ["AND", src] => binop(0xa0, src, &mut data),
                    ["XOR", src] => binop(0xa8, src, &mut data),
                    ["OR", src] =>  binop(0xb0, src, &mut data),
                    ["CP", src] =>  binop(0xb8, src, &mut data),

                    ["INC", dst] => match reg8(dst) {
                        Some(r) => data.push(0x04+0x08*r),
                        None => match reg16(dst) {
                            Some(r) => data.push(0x03+r),
                            None => panic!("illegal target for INC"),
                    }}
                    ["DEC", dst] => match reg8(dst) {
                        Some(r) => data.push(0x05+0x08*r),
                        None => match reg16(dst) {
                            Some(r) => data.push(0x0b+r),
                            None => panic!("illegal target for DEC"),
                    }}


                    ["RLC", src] =>  {data.push(0xcb); binop(0x00, src, &mut data)},
                    ["RRC", src] =>  {data.push(0xcb); binop(0x08, src, &mut data)},
                    ["RL", src] =>   {data.push(0xcb); binop(0x10, src, &mut data)},
                    ["RR", src] =>   {data.push(0xcb); binop(0x18, src, &mut data)},
                    ["SLA", src] =>  {data.push(0xcb); binop(0x20, src, &mut data)},
                    ["SRA", src] =>  {data.push(0xcb); binop(0x28, src, &mut data)},
                    ["SWAP", src] => {data.push(0xcb); binop(0x30, src, &mut data)},
                    ["SRL", src] =>  {data.push(0xcb); binop(0x38, src, &mut data)},

                    ["BIT", "0", src] => {data.push(0xcb); binop(0x40, src, &mut data)},
                    ["BIT", "1", src] => {data.push(0xcb); binop(0x48, src, &mut data)},
                    ["BIT", "2", src] => {data.push(0xcb); binop(0x50, src, &mut data)},
                    ["BIT", "3", src] => {data.push(0xcb); binop(0x58, src, &mut data)},
                    ["BIT", "4", src] => {data.push(0xcb); binop(0x60, src, &mut data)},
                    ["BIT", "5", src] => {data.push(0xcb); binop(0x68, src, &mut data)},
                    ["BIT", "6", src] => {data.push(0xcb); binop(0x70, src, &mut data)},
                    ["BIT", "7", src] => {data.push(0xcb); binop(0x78, src, &mut data)},

                    ["NOP"] => data.push(0x00),
                    ["CCF"] => data.push(0x3f),
                    ["CPL"] => data.push(0x2f),
                    ["HALT"] => data.push(0x76),
                    ["STOP"] => data.push(0x10),
                    ["STOP", "0"] => {data.push(0x10); data.push(0x00)},
                    ["RETI"] => data.push(0xd9),
                    ["DI"] => data.push(0xf3),
                    ["EI"] => data.push(0xfb),
                    ["RLCA"] => data.push(0x07),
                    ["RRCA"] => data.push(0x0f),
                    ["RLA"] => data.push(0x17),
                    ["RRA"] => data.push(0x1f),
                    ["DAA"] => data.push(0x27),
                    ["SCF"] => data.push(0x37),

                    ["JR", cond, addr] => jump(0x20+cond_offset(cond), addr, true, &mut data, &mut fixes),
                    ["JR",       addr] => jump(0x18, addr, true, &mut data, &mut fixes),
                    ["JP", cond, addr] => jump(0xc2+cond_offset(cond), addr, false, &mut data, &mut fixes),
                    ["JP",       addr] => jump(0xc3, addr, false, &mut data, &mut fixes),
                    ["CALL", cond, addr] => jump(0xc4+cond_offset(cond), addr, false, &mut data, &mut fixes),
                    ["CALL",       addr] => jump(0xcd, addr, false, &mut data, &mut fixes),

                    ["RET", cond] => data.push(0xC0+cond_offset(cond)),
                    ["RET",     ] => data.push(0xC9),

                    ["POP",  dst] => data.push(0xC1+reg16(dst).unwrap()),
                    ["PUSH", src] => data.push(0xC5+reg16(src).unwrap()),

                    _ => {
                        if components[0] == ".DB" {
                            for x in components[1..].iter() {
                                data.push(imm8(x).expect("could not parse byte in .DB"));
                            }
                        }
                        else {
                            panic!("Can not parse line {}", line);
                        }
                    }
                }
        }
    }

    for (pos, addr, relative) in fixes.iter() {
        let tgt = match labels.get(addr) {
            Some(x) => x,
            None => panic!("label {} was not defined", addr)
        };
        if *relative {
            let offset = if tgt < pos {(256+tgt-pos) as u8} else {(tgt-pos) as u8};
            data[pos-1] = offset;
        }
        else {
            let [h,l] = (*tgt as u16).to_be_bytes();
            data[pos-2] = l;
            data[pos-1] = h;
        }
    }

    println!("Labels:");
    for (label, address) in labels.iter() {
        println!("0x{:04x}: {}", address, label);
    }

    let filename_out = &args[2];
    let mut f_out = File::create(filename_out).expect("could not open file for writing");
    f_out.write_all(&data).expect("error writing data.");
}
