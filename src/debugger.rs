use std::collections::HashSet;
use std::io;
use std::io::Write;

use crate::cpu::*;
use crate::instructions;

struct Debugger {
    cpu: Cpu,
    breakpoints: HashSet<u16>,
}

enum DbgCommand {
    Error,
    Continue,
    SingleStep,
    SetBreakpoint (u16),
    ClearBreakpoint (u16),
    ToggleTrace,
}

fn parseCommand(line: &String) -> DbgCommand {
    use DbgCommand::*;
    let mut iter = line.split_whitespace();
    match iter.next() {
        Some("c") => Continue,
        Some("s") => SingleStep,
        Some("b") => match iter.next() {
            Some(word) => match u16::from_str_radix(word, 16) {
                Ok(addr) => SetBreakpoint(addr),
                _ => Error,
            },
            _ => Error,
        }
        Some("cl") => match iter.next() {
            Some(word) => match u16::from_str_radix(word, 16) {
                Ok(addr) => ClearBreakpoint(addr),
                _ => Error,
            },
            _ => Error,
        }
        Some("t") => ToggleTrace,
        _ => Error,
    }
}

impl Debugger {
    fn new(cpu:Cpu) -> Debugger{
        Debugger {cpu, breakpoints: HashSet::new(),}
    }

    fn run(&mut self) {
        let mut singleStep = true;
        let mut trace = false;
        loop {
            loop {
                if singleStep | self.breakpoints.contains(&self.cpu.pc) {
                    break;
                }
                if trace {
                    println!("{}  {}", dis_instr(&self.cpu.mmu, self.cpu.pc), cpustate(&self.cpu));
                }
                self.cpu.step();
            }

            println!("{}  {}", dis_instr(&self.cpu.mmu, self.cpu.pc), cpustate(&self.cpu));
            print!("rboy dbg> ");
            io::stdout().flush();

            let mut line = String::new();
            io::stdin().read_line(&mut line).expect("Could not read command from stdin.");
            use DbgCommand::*;
            match parseCommand(&line) {
                Continue => {singleStep = false; self.cpu.step();},
                SingleStep => {singleStep = true; self.cpu.step();},
                SetBreakpoint(addr) => {self.breakpoints.insert(addr);},
                ClearBreakpoint(addr) => {self.breakpoints.remove(&addr);},
                ToggleTrace => {trace = !trace; println!("trace is {}.", if trace {"on"} else {"off"});},
                Error => {println!("DebuggerCommands:\n  c: continue\n  s: single step\n  b addr: set breakpoint\n  cl addr: clear breakpoint");},
            }
        }
    }

}

fn dis_instr(mmu:&Mmu, addr:u16) -> String {
    let mut instr = &instructions::instructions[mmu.read(addr) as usize];
    if instr.operation == instructions::Operation::PREFIX {
        instr = &instructions::instructions[mmu.read(addr+1) as usize + 0x100];
    }
    let instr = instr;

    match instr.length {
        2 => format!("0x{:04x}: {:02x}{:02x}    {:11} 0x{:02x}  ",
                    addr, mmu.read(addr), mmu.read(addr+1), instr.mnemo, mmu.read(addr+1)),
        3 => format!("0x{:04x}: {:02x}{:02x}{:02x}  {:11} 0x{:02x}{:02x}",
                    addr, mmu.read(addr), mmu.read(addr+1), mmu.read(addr+2), instr.mnemo, mmu.read(addr+2), mmu.read(addr+1)),
        _ => format!("0x{:04x}: {:02x}      {:11}       ",
                    addr, mmu.read(addr), instr.mnemo),
    }
}

fn cpustate(cpu:&Cpu) -> String {
    format!("A:{:02x} B:{:02x} C:{:02x} D:{:02x} E:{:02x} HL:{:02x}{:02x}->{:02x} SP:{:04x}->{:02x} {}{}{}{}",
              cpu.a, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l,
              cpu.mmu.read(word(cpu.h, cpu.l)), cpu.sp, cpu.mmu.read(cpu.sp),
              if FLAG_Z & cpu.f != 0 {"Z"} else {"-"},
              if FLAG_N & cpu.f != 0 {"N"} else {"-"},
              if FLAG_H & cpu.f != 0 {"H"} else {"-"},
              if FLAG_C & cpu.f != 0 {"C"} else {"-"},
          )
}

pub fn main() {
    let mut mmu = Mmu::new();
    mmu.load("DMG_ROM.bin", 0);
    let mut cpu = Cpu::new(mmu);
    let mut dbg = Debugger::new(cpu);
    dbg.run();
}