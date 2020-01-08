use std::collections::HashSet;
use std::io;
use std::io::Write;

use crate::cpu::*;
use crate::ppu::Ppu;
use crate::instructions;

extern crate image as im;
use im::{ImageBuffer, Rgba};


pub struct Debugger {
    cpu: Cpu,
    ppu: Ppu,
    breakpoints: HashSet<u16>,
    trace: bool,
    running: bool,
}

enum DbgCommand {
    Error,
    Continue,
    SingleStep,
    SetBreakpoint (u16),
    ClearBreakpoint (u16),
    ToggleTrace,
    Quit,
    DumpMemory (u16),
}

fn parse_command(line: &String) -> DbgCommand {
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
        Some("q") => Quit,
        Some("d") => match iter.next() {
            Some(word) => match u16::from_str_radix(word, 16) {
                Ok(addr) => DumpMemory(addr),
                _ => Error,
            },
            _ => Error,
        }
        _ => Error,
    }
}

impl Debugger {
    pub fn new(cpu:Cpu, ppu:Ppu) -> Debugger{
        Debugger {cpu, ppu, breakpoints: HashSet::new(), trace:true, running: false}
    }

    pub fn interact(&mut self, lcd: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, max_cycles:isize, buttons:u8) -> isize {
        self.cpu.mmu.set_buttons(buttons);
        if self.running {
            self.run_to_breakpoint(lcd, false, max_cycles)
        }
        else {
            println!("{}  {}  {}", dis_instr(&self.cpu.mmu, self.cpu.pc), cpustate(&self.cpu), ppustate(&self.ppu, &self.cpu.mmu));
            print!("rboy dbg> ");
            io::stdout().flush().expect("error on stdout.flush");

            let mut line = String::new();
            io::stdin().read_line(&mut line).expect("Could not read command from stdin.");

            use DbgCommand::*;
            match parse_command(&line) {
                Continue => self.run_to_breakpoint(lcd, false, max_cycles),
                SingleStep => self.run_to_breakpoint(lcd, true, max_cycles),
                SetBreakpoint(addr) => {self.breakpoints.insert(addr);0},
                ClearBreakpoint(addr) => {self.breakpoints.remove(&addr);0},
                ToggleTrace => {
                    self.trace = !self.trace;
                    println!("trace is {}.", if self.trace {"on"} else {"off"});
                    0
                },
                Quit => 0,
                DumpMemory(addr) => {
                    let start = if addr < 0xff00 {addr} else {0xff00};
                    for i in 0..16 {
                        let md = |a| format!("{:02x}{:02x}{:02x}{:02x}",
                            self.cpu.mmu.read(a),
                            self.cpu.mmu.read(a+1),
                            self.cpu.mmu.read(a+2),
                            self.cpu.mmu.read(a+3));
                        let base = start + 16*i;
                        println!("{:04x}  {} {}  {} {}",
                            base,
                            md(base),
                            md(base+4),
                            md(base+8),
                            md(base+12));
                    }
                    0
                },
                Error => {
                    println!("DebuggerCommands:\n  c: continue\n  s: single step\n  b addr: set breakpoint\n  cl addr: clear breakpoint");
                    0
                },
            }
        }
    }

    fn run_to_breakpoint(&mut self, lcd: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, single_step: bool, max_cycles:isize) -> isize {
        let mut total_cycles = 0;
        let mut max_cycles = max_cycles;
        self.running = true;
        while max_cycles > 0 {
            let cycles = self.cpu.step();
            total_cycles += cycles;
            max_cycles -= cycles;
            self.ppu.run_for(&mut self.cpu.mmu, lcd, cycles);
            self.cpu.mmu.tick(cycles);

            if single_step | self.breakpoints.contains(&self.cpu.pc) {
                self.running = false;
                break;
            }

            if self.trace {
                println!("{}  {}  {}", dis_instr(&self.cpu.mmu, self.cpu.pc), cpustate(&self.cpu), ppustate(&self.ppu, &self.cpu.mmu));
            }
        }
        total_cycles
    }
}

fn dis_instr(mmu:&Mmu, addr:u16) -> String {
    let mut instr = &instructions::INSTRUCTIONS[mmu.read(addr) as usize];
    if instr.operation == instructions::Operation::PREFIX {
        instr = &instructions::INSTRUCTIONS[mmu.read(addr+1) as usize + 0x100];
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
    format!("A:{:02x} B:{:02x} C:{:02x} D:{:02x} E:{:02x} HL:{:02x}{:02x}->{:02x} SP:{:04x}->{:02x} {}{}{}{}{} IF:{:02x} IE:{:02x}  ",
              cpu.a, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l,
              cpu.mmu.read(word(cpu.h, cpu.l)), cpu.sp, cpu.mmu.read(cpu.sp),
              if FLAG_Z & cpu.f != 0 {"Z"} else {"-"},
              if FLAG_N & cpu.f != 0 {"N"} else {"-"},
              if FLAG_H & cpu.f != 0 {"H"} else {"-"},
              if FLAG_C & cpu.f != 0 {"C"} else {"-"},
              if cpu.ie {"I"} else {"-"},
              cpu.mmu.read(0xff0f),
              cpu.mmu.read(0xffff),
          )
}

fn ppustate(ppu:&Ppu, mmu:&Mmu) -> String {
    format!("  x={} y={} mode={} cycles_left={}",
        ppu.x,
        mmu.read(0xff44),
        ppu.mode,
        ppu.cycles_left,
    )
}
