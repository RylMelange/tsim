use std::{
    fmt::Write,
    fs::{read_to_string, write},
    path::Path,
};

use bitflags::bitflags;
use ternary_tools::helper::*;

#[derive(Debug, Clone, Copy)]
enum RegionType {
    Ram,
    Sto,
}

pub struct Machine {
    pc: i16,
    a: i16,
    b: i16,
    sar: i16,
    srr: i16,
    paused: bool,
    ram: Vec<i16>,
    sto: Vec<i16>,
    cycles: u32,
}

bitflags! {
    #[derive(PartialEq)]
    pub struct States: u8 {
        const ALL = 0b11111111;
        const RAM = 0b01000000;
        const STO = 0b10000000;
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            pc: 0,
            a: 0,
            b: 0,
            sar: -HALF_WORD,
            srr: 0,
            paused: false,
            ram: vec![0; RAM_SIZE],
            sto: vec![0; STO_SIZE],
            cycles: 0,
        }
    }
}

impl Machine {
    fn run(&mut self, limit: Option<usize>) -> Result<(), String> {
        let mut step = 0usize;
        loop {
            if self.paused {
                break;
            }

            if let Some(max) = limit {
                if step >= max {
                    break;
                }
            }

            self.step()?;
            step += 1;
        }
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), String> {
        let opcode = self.advance_pc()?;
        let op = Op::from_i16(opcode);
        self.cycles += 1;
        println!("{}: Running operation: {:?}", self.cycles, op);

        match op {
            // TODO: maybe introduce set_a(num) to avoid writing % HALF_WORD all over the place
            Op::Nop => {}
            Op::Hlt => {
                self.paused = true;
            }
            Op::Cla => {
                self.a = 0;
                self.b = 0;
                self.srr = 0;
            }
            Op::Lra => {
                let operand = self.advance_pc()?;
                self.a = self.read_word(RegionType::Ram, operand)?
            }
            Op::Sra => {
                let operand = self.advance_pc()?;
                self.write_word(RegionType::Ram, operand, self.a)?
            }
            Op::Lsa => {
                self.a = self.read_word(RegionType::Sto, self.sar)?;
            }
            Op::Ssa => self.write_word(RegionType::Sto, self.sar, self.a)?,
            Op::Lpa => self.a = self.sar,
            Op::Spa => self.sar = self.a,
            Op::Lia => self.a = self.advance_pc()?,
            Op::Neg => self.a = -self.a,
            Op::Abs => self.a = self.a.abs(),
            Op::Adv => self.a = if self.a >= 0 { self.a + 1 } else { self.a - 1 },

            Op::Ima => {
                let addr = self.advance_pc()?;
                let amnt = self.read_word(RegionType::Ram, addr)?;
                self.a = if self.a >= 0 {
                    (self.a + amnt) % HALF_WORD
                } else {
                    (self.a - amnt) % HALF_WORD
                }
            }
            Op::Dma => {
                let addr = self.advance_pc()?;
                let amnt = self.read_word(RegionType::Ram, addr)?;
                self.a = if self.a >= 0 {
                    (self.a - amnt) % HALF_WORD
                } else {
                    (self.a + amnt) % HALF_WORD
                }
            }
            Op::Imi => {
                let amnt = self.advance_pc()?;
                self.a = if self.a >= 0 {
                    (self.a + amnt) % HALF_WORD
                } else {
                    (self.a - amnt) % HALF_WORD
                }
            }
            Op::Dmi => {
                let amnt = self.advance_pc()?;
                self.a = if self.a >= 0 {
                    (self.a - amnt) % HALF_WORD
                } else {
                    (self.a + amnt) % HALF_WORD
                }
            }

            Op::Mul => {
                self.b = self.advance_pc()?;
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a * self.b) % HALF_WORD
            }
            Op::Adi => {
                self.b = self.advance_pc()?;
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a + self.b) % HALF_WORD;
            }
            Op::Add => {
                let addr = self.advance_pc()?;
                self.b = self.read_word(RegionType::Ram, addr)?;
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a + self.b) % HALF_WORD;
            }
            Op::Sbi => {
                self.b = self.advance_pc()?;
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a - self.b) % HALF_WORD;
            }
            Op::Sbb => {
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a - self.b) % HALF_WORD;
            }
            Op::Sub => {
                let addr = self.advance_pc()?;
                self.b = self.read_word(RegionType::Ram, addr)?;
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a - self.b) % HALF_WORD;
            }
            Op::Mli => {
                self.b = self.advance_pc()?;
                // TODO: this is probably not accurate, fix someday
                self.a = ((self.a as i32 * self.b as i32) % HALF_WORD as i32) as i16;
            }
            Op::Tmi => {
                self.b = self.advance_pc()?;
                // TODO: this is probably not accurate, fix someday
                self.a = tritwise_mul(self.a, self.b) % HALF_WORD;
            }
            Op::Jmp => self.pc = self.advance_pc()?,
            Op::Jnz => {
                let target = self.advance_pc()?;
                if self.a != 0 {
                    self.pc = target;
                }
            }
            Op::Jmz => {
                let target = self.advance_pc()?;
                if self.a == 0 {
                    self.pc = target;
                }
            }
            Op::Jnn => {
                let target = self.advance_pc()?;
                if self.a >= 0 {
                    self.pc = target;
                }
            }
            Op::Jms => {
                let target = self.advance_pc()?;
                if self.a > 0 {
                    self.pc = target;
                } else if self.a < 0 {
                    self.pc = -target;
                }
            }
            Op::Rte => {
                let target = self.advance_pc()?;
                self.srr = self.pc;
                self.pc = target;
            }
            Op::Rtr => self.pc = self.srr,
            Op::Asp => {
                self.sar = if self.sar < 0 {
                    -self.sar + 1
                } else {
                    -self.sar
                }
            }
            Op::Shl => {
                self.a = self.a * 3 % HALF_WORD;
            }
            Op::Shr => {
                if self.a == 0 {
                    let trits = i16_to_trit(self.a);
                    self.a = trit_to_i16(&trits[1..]);
                }
            }
            Op::Unknown => {
                self.paused = true;
                eprintln!(
                    "Unknown opcode at {}: {}",
                    i16_to_code(self.pc - 1),
                    i16_to_code(opcode)
                )
            }
        }

        Ok(())
    }

    pub fn load_state(&mut self, path: &Path) -> Result<(), String> {
        let text = read_to_string(path).map_err(|e| e.to_string())?;
        let mut lines = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty());

        let header = lines.next().ok_or_else(|| "Empty file".to_string())?;
        if header != "TSIM2" {
            return Err("Unrecognised state file header (expected TSIM2)".to_string());
        }

        while let Some(line) = lines.next() {
            if line.eq_ignore_ascii_case("RAM") {
                for i in 0..RAM_SIZE {
                    let code = lines
                        .next()
                        .ok_or_else(|| "Unexpected end of file while reading RAM".to_string())?;
                    self.ram[i] = code_to_i16(code)?;
                }
            } else if line.eq_ignore_ascii_case("STO") {
                for i in 0..STO_SIZE {
                    let code = lines
                        .next()
                        .ok_or_else(|| "Unexpected end of file while reading STO".to_string())?;
                    self.sto[i] = code_to_i16(code)?;
                }
            } else {
                let mut p = line.split_whitespace();
                let key = p.next().ok_or_else(|| "Invalid state line".to_string())?;
                let val = p
                    .next()
                    .ok_or_else(|| format!("Missing value for {}", key).to_string())?;

                match key.to_lowercase().as_str() {
                    "pc" => self.pc = code_to_i16(val)?,
                    "a" => self.a = code_to_i16(val)?,
                    "b" => self.b = code_to_i16(val)?,
                    "sar" => self.sar = code_to_i16(val)?,
                    "paused" => self.paused = val.eq_ignore_ascii_case("true"),
                    _ => return Err(format!("Unknown state key: {}", key)),
                }
            }
        }
        Ok(())
    }

    fn save_state(&mut self, to_save: States, path: &Path) -> Result<(), String> {
        let mut state = String::new();
        state.push_str("TSIM2\n");

        if to_save == States::ALL {
            state.push_str(&format!("pc {}\n", i16_to_code(self.pc)));
            state.push_str(&format!("a {}\n", i16_to_code(self.a)));
            state.push_str(&format!("b {}\n", i16_to_code(self.b)));
            state.push_str(&format!("sar {}\n", i16_to_code(self.sar)));
            state.push_str(&format!(
                "paused {}\n",
                if self.paused { "true" } else { "false" }
            ));
        };

        if to_save.intersects(States::RAM) {
            state.push_str("RAM\n");
            for val in &self.ram {
                state.push_str(&format!("{}\n", i16_to_code(*val)));
            }
        }

        if to_save.intersects(States::STO) {
            state.push_str("STO\n");
            for val in &self.sto {
                state.push_str(&format!("{}\n", i16_to_code(*val)));
            }
        }

        write(path, state).map_err(|e| e.to_string())
    }

    fn write_word(&mut self, region: RegionType, addr: i16, value: i16) -> Result<(), String> {
        // TODO: this is only right if all regions have the same size, WORD_SIZE
        let idx = (addr + HALF_WORD) as usize;
        if let Some(word) = match region {
            RegionType::Ram => self.ram.get_mut(idx),
            RegionType::Sto => self.sto.get_mut(idx),
        } && value.abs() <= HALF_WORD
        {
            *word = value;
            Ok(())
        } else {
            Err(format!("Address {} or value {} out of range!", addr, value))
        }
    }

    fn write_stream(&mut self, region: RegionType, start: i16, data: &str) -> Result<(), String> {
        let mut values = Vec::new();
        for c in data.chars() {
            values.push(char_to_digit(c).ok_or_else(|| format!("Invalid symbol '{}'", c))?);
        }

        let mut addr = start;
        let mut i = 0usize;
        while i < values.len() {
            let hi = values[i];
            let lo = if i + 1 < values.len() {
                values[i + 1]
            } else {
                0
            };

            let word = hi * SIP_SIZE + lo;
            self.write_word(region, addr, word)?;

            addr += 1;
            if addr > WORD_SIZE {
                return Err("address limit reached!".to_string());
            }
            i += 2;
        }

        Ok(())
    }

    fn advance_pc(&mut self) -> Result<i16, String> {
        let word = self.read_word(RegionType::Ram, self.pc)?;
        if self.pc >= 0 {
            self.pc += 1;
            if self.pc > HALF_WORD {
                self.pc = -HALF_WORD;
                self.paused = true;
            }
        } else {
            self.pc -= 1;
            if self.pc < -HALF_WORD {
                self.pc = HALF_WORD;
                self.paused = true;
            }
        }
        Ok(word)
    }
    fn read_word(&mut self, region: RegionType, addr: i16) -> Result<i16, String> {
        // TODO: this is only right if all regions have the same size, WORD_SIZE
        let idx = (addr + HALF_WORD) as usize;
        if let Some(word) = match region {
            RegionType::Ram => self.ram.get_mut(idx),
            RegionType::Sto => self.sto.get_mut(idx),
        } {
            Ok(*word)
        } else {
            Err(format!("Address {} out of range!", addr))
        }
    }

    fn dump_region(&mut self, region: RegionType, start: i16, end: i16) -> Result<(), String> {
        let mut line = String::new();
        let mut addr = start;

        // TODO: add a heading for the top?

        while addr <= end {
            line.clear();
            // write!(&mut line, "{}*:", i16_to_code(addr).chars().next().unwrap()).unwrap();
            write!(&mut line, "{}:", i16_to_code(addr)).unwrap();

            for offset in 0..SIP_SIZE {
                let cur = addr + offset;
                if cur > end {
                    break;
                }
                let word = self.read_word(region, cur)?;
                write!(&mut line, " {}", i16_to_code(word)).unwrap();
            }
            println!("{}", line);

            addr += SIP_SIZE;
            if addr > end {
                break;
            }
        }
        Ok(())
    }

    fn print_state(&mut self, all: bool) {
        println!("PC: {} ({})", self.pc, i16_to_code(self.pc));
        println!("A: {} ({})", self.a, i16_to_code(self.a));
        println!("B: {} ({})", self.b, i16_to_code(self.b));
        println!("SAR: {} ({})", self.sar, i16_to_code(self.sar));
        println!("Paused: {}", self.paused);

        if all {
            println!();
            println!("RAM:");
            self.dump_region(RegionType::Ram, -HALF_WORD, HALF_WORD)
                .expect("error in dumping ram...");
            println!();
            println!("STO:");
            self.dump_region(RegionType::Sto, -HALF_WORD, HALF_WORD)
                .expect("error in dumping sto...");
        }
    }
}

#[derive(Debug)]
enum Op {
    Nop,
    Hlt,
    Cla,

    Asp,

    Lra,
    Sra,
    Lsa,
    Ssa,
    Lpa,
    Spa,
    Lia,

    Jms,
    Jmp,
    Jnz,
    Jmz,
    Jnn,
    Rte,
    Rtr,

    Abs,
    Neg,
    Adv,
    Ima,
    Dma,
    Imi,
    Dmi,

    Adi,
    Sbi,
    Mli,
    Tmi,
    Add,
    Sub,
    Mul,
    Sbb,

    Shl,
    Shr,

    Unknown,
}

impl Op {
    fn from_i16(v: i16) -> Self {
        match v {
            -9812 => Self::Nop,
            6149 => Self::Hlt,
            2512 => Self::Cla,

            502 => Self::Asp,

            8506 => Self::Lra,
            -6074 => Self::Sra,
            8533 => Self::Lsa,
            -6047 => Self::Ssa,
            8452 => Self::Lpa,
            -6128 => Self::Spa,
            8992 => Self::Lia,

            7633 => Self::Jms,
            7630 => Self::Jmp,
            7640 => Self::Jmz,
            6938 => Self::Jnz,
            6926 => Self::Jnn,
            -6745 => Self::Rte,
            -6759 => Self::Rtr,

            775 => Self::Abs,
            -9335 => Self::Neg,
            832 => Self::Adv,
            6913 => Self::Ima,
            3268 => Self::Dma,
            6921 => Self::Imi,
            3276 => Self::Dmi,

            846 => Self::Adi,
            -5769 => Self::Sbi,
            9810 => Self::Mli,
            -4743 => Self::Tmi,
            841 => Self::Add,
            -5992 => Self::Sub,
            9327 => Self::Mul,
            -5776 => Self::Sbb,

            -5604 => Self::Shl,
            -5625 => Self::Shr,

            _ => Self::Unknown,
        }
    }
}

fn tritwise_mul(a: i16, b: i16) -> i16 {
    let ta = i16_to_trit(a);
    let tb = i16_to_trit(b);

    let len = ta.len().max(tb.len());
    let mut result = Vec::with_capacity(len);

    for i in 0..len {
        let x = *ta.get(i).unwrap_or(&0);
        let y = *tb.get(i).unwrap_or(&0);
        result.push(x * y);
    }

    trit_to_i16(&result)
}
