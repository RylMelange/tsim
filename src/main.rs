use clap::Parser;
use std::fmt::Write as _;
use std::{
    fs::{self, read_to_string},
    io::{self, Write},
    path::{Path, PathBuf},
};

const SIP_SIZE: i16 = 27;
const SIP_PER_WORD: u32 = 2;
const WORD_SIZE: i16 = SIP_SIZE.pow(SIP_PER_WORD);
const HALF_WORD: i16 = WORD_SIZE / 2;
const RAM_SIZE: usize = WORD_SIZE as usize;
const STO_SIZE: usize = WORD_SIZE as usize;

#[derive(Parser, Debug)]
#[command(name = "tsim", version, about = "A binary VM", long_about = None)]
struct Args {
    /// Load a state file
    #[arg(short, long)]
    state: Option<PathBuf>,
}

struct Machine {
    pc: i16,
    a: i16,
    b: i16,
    sar: i16,
    srr: i16,
    paused: bool,
    ram: Vec<i16>,
    sto: Vec<i16>,
}

#[derive(Debug, Clone, Copy)]
enum RegionType {
    Ram,
    Sto,
}

enum Op {
    Nop,
    Hlt,
    Cla,

    Lra,
    Sra,
    Lsa,
    Ssa,
    Lpa,
    Spa,
    Lia,

    Mul,
    Add,
    Sub,

    Jmp,
    Jnz,
    Jms,
    Rte,
    Rtr,

    Unknown,
}

impl Op {
    fn from_i16(v: i16) -> Self {
        match v {
            0 => Self::Nop,
            3 => Self::Cla,
            8 => Self::Hlt,
            28 => Self::Lra,
            26 => Self::Sra,
            37 => Self::Lsa,
            35 => Self::Ssa,
            40 => Self::Lpa,
            38 => Self::Spa,
            36 => Self::Lia,
            364 => Self::Mul,
            352 => Self::Add,
            343 => Self::Sub,
            283 => Self::Jmp,
            257 => Self::Jnz,
            262 => Self::Jms,
            -238 => Self::Rte,
            -252 => Self::Rtr,
            _ => Self::Unknown,
        }
    }
}

fn main() {
    let args = Args::parse();

    let mut machine = Machine::default();

    if let Some(path) = args.state {
        match machine.load_state(&path) {
            Ok(_) => {
                println!("Loaded state!");
            }
            Err(e) => {
                eprintln!("Error loading {}: {}", path.display(), e);
            }
        }
    };

    repl(&mut machine);
}

fn repl(machine: &mut Machine) {
    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("\x1b[1;31mtsim> \x1b[0m");
        let _ = io::stdout().flush();
        line.clear();

        match stdin.read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }

        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }
        if let Err(e) = handle_command(machine, trimmed) {
            eprintln!("{}", e);
        }
    }
}

fn handle_command(machine: &mut Machine, input: &str) -> Result<(), String> {
    let mut args: Vec<&str> = input.trim().split_whitespace().collect();
    if args.is_empty() {
        return Ok(());
    }

    let cmd = args.remove(0).to_lowercase();
    match cmd.as_str() {
        "q" => std::process::exit(0),
        "state" => {
            if args.is_empty() {
                machine.print_state(false);
            } else if args[0].eq_ignore_ascii_case("all") {
                machine.print_state(true);
            } else {
                return Err("Usage: state [all]".to_string());
            }
            Ok(())
        }
        "test" => {
            println!("{}", code_to_i16(args[0])?);
            Ok(())
        }
        "step" | "\'" => {
            machine.paused = false;
            machine.step()?;
            machine.print_state(false);
            Ok(())
        }
        "run" => {
            machine.paused = false;
            match args.len() {
                0 => machine.run(None)?,
                1 => machine.run(Some(
                    args[0]
                        .parse()
                        .map_err(|_| format!("Invalid number: {}", args[0]))?,
                ))?,
                _ => return Err("Usage: run [steps]".to_string()),
            }
            machine.print_state(false);
            Ok(())
        }
        "dump" => {
            if args.is_empty() {
                return Err("Usage: dump <ram|sto> [start end]".to_string());
            }
            let region = match args[0].to_lowercase().as_str() {
                "ram" => RegionType::Ram,
                "sto" => RegionType::Sto,
                _ => {
                    return Err(format!("Error: expected RAM or STO"));
                }
            };
            let (start, end) = match args.len() {
                // TODO: this is only right if all regions have the same size, WORD_SIZE
                1 => (-(HALF_WORD), HALF_WORD),
                2 => {
                    let s = code_to_i16(args[1])?;
                    (s, s)
                }
                3 => {
                    let s = code_to_i16(args[1])?;
                    let e = code_to_i16(args[2])?;
                    if s <= e { (s, e) } else { (e, s) }
                }
                _ => return Err("Usage: dump <ram|sto> [start end]".to_string()),
            };
            machine.dump_region(region, start, end)
        }
        "pc" => {
            if args.len() != 1 {
                return Err("Usage: pc <value>".to_string());
            }
            machine.pc = code_to_i16(args[0])?;
            machine.paused = false;
            Ok(())
        }
        "a" => {
            if args.len() != 1 {
                return Err("Usage: a <value>".to_string());
            }
            machine.a = code_to_i16(args[0])?;
            Ok(())
        }
        "b" => {
            if args.len() != 1 {
                return Err("Usage: b <value>".to_string());
            }
            machine.b = code_to_i16(args[0])?;
            Ok(())
        }
        "sar" => {
            if args.len() != 1 {
                return Err("Usage: sar <value>".to_string());
            }
            machine.sar = code_to_i16(args[0])?;
            Ok(())
        }
        "write" => {
            // TODO: maybe accept assembly codes as well
            if args.len() < 2 {
                return Err(format!("Usage: write <ram|sto> <addr>"));
            }

            let region = match args[0].to_lowercase().as_str() {
                "ram" => RegionType::Ram,
                "sto" => RegionType::Sto,
                _ => {
                    return Err(format!("Error: expected RAM or STO"));
                }
            };

            let addr = code_to_i16(args[1])?;

            // TODO: add some way to abort
            print!("Enter data to write: ");

            let _ = io::stdout().flush();
            let mut buf = String::new();
            io::stdin().read_line(&mut buf).map_err(|e| e.to_string())?;

            let t = buf.trim();
            if t.is_empty() {
                println!("Cancelled.");
                return Ok(());
            }
            let data = t.to_string();

            machine.write_stream(region, addr, &data)
        }
        "load" => {
            if args.len() == 1 {
                machine.load_state(PathBuf::from(&args[0]).as_path())
            } else {
                return Err(format!("Usage: load <file>"));
            }
        }
        "save" => {
            if args.len() == 1 {
                machine.save_state(PathBuf::from(&args[0]).as_path())
            } else {
                Err(format!("Usage: save <file>"))
            }
        }
        _ => {
            println!("Unknown command: {}", cmd);
            println!("TODO: write available commands here");
            Ok(())
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            pc: 0,
            a: 0,
            b: 0,
            sar: 0,
            srr: 0,
            paused: false,
            ram: vec![0; RAM_SIZE],
            sto: vec![0; STO_SIZE],
        }
    }
}

impl Machine {
    fn run(&mut self, limit: Option<usize>) -> Result<(), String> {
        let mut steps = 0usize;
        loop {
            if self.paused {
                break;
            }

            if let Some(max) = limit {
                if steps >= max {
                    break;
                }
            }

            self.step()?;
            steps += 1;
        }
        Ok(())
    }

    fn step(&mut self) -> Result<(), String> {
        let opcode = self.advance_pc()?;
        let op = Op::from_i16(opcode);

        match op {
            Op::Nop => {}
            Op::Hlt => self.paused = true,
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
            // TODO: LSA is assymetric with SSA, maybe add 2 more instructions?
            Op::Lsa => {
                self.a = self.read_word(RegionType::Sto, self.sar)?;
                self.sar = (self.sar + 1) % HALF_WORD
            }
            Op::Ssa => {
                let operand = self.advance_pc()?;
                self.write_word(RegionType::Sto, operand, self.a)?
            }
            Op::Lpa => self.a = self.sar,
            Op::Spa => self.sar = self.a,
            Op::Lia => self.a = self.advance_pc()?,
            Op::Mul => {
                let operand = self.advance_pc()?;
                self.b = self.read_word(RegionType::Ram, operand)?;
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a * self.b) % HALF_WORD
            }
            Op::Add => {
                let operand = self.advance_pc()?;
                self.b = self.read_word(RegionType::Ram, operand)?;
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a + self.b) % HALF_WORD;
            }
            Op::Sub => {
                let operand = self.advance_pc()?;
                self.b = self.read_word(RegionType::Ram, operand)?;
                // TODO: this is probably not accurate, fix someday
                self.a = (self.a - self.b) % HALF_WORD;
            }
            Op::Jmp => self.pc = self.advance_pc()?,
            Op::Jnz => {
                let target = self.advance_pc()?;
                if self.a != 0 {
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
                self.srr = self.pc + 1;
                self.pc = self.advance_pc()?;
            }
            Op::Rtr => self.pc = self.srr,
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

    fn load_state(&mut self, path: &Path) -> Result<(), String> {
        let text = read_to_string(path).map_err(|e| e.to_string())?;
        let mut lines = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty());

        let header = lines.next().ok_or_else(|| "Empty file".to_string())?;
        if header != "TSIM1" {
            return Err("Unrecognised state file header (expected TSIM1)".to_string());
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
                    "paused" => self.paused = val.eq_ignore_ascii_case("true"),
                    _ => return Err(format!("Unknown state key: {}", key)),
                }
            }
        }
        Ok(())
    }

    fn save_state(&mut self, path: &Path) -> Result<(), String> {
        let mut state = String::new();
        state.push_str("TSIM1\n");
        state.push_str(&format!("pc {}\n", i16_to_code(self.pc)));
        state.push_str(&format!("a {}\n", i16_to_code(self.a)));
        state.push_str(&format!("b {}\n", i16_to_code(self.b)));
        state.push_str(&format!(
            "paused {}\n",
            if self.paused { "true" } else { "false" }
        ));

        state.push_str("RAM\n");
        for val in &self.ram {
            state.push_str(&format!("{}\n", i16_to_code(*val)));
        }

        state.push_str("STO\n");
        for val in &self.sto {
            state.push_str(&format!("{}\n", i16_to_code(*val)));
        }

        fs::write(path, state).map_err(|e| e.to_string())
    }

    fn write_word(&mut self, region: RegionType, addr: i16, value: i16) -> Result<(), String> {
        // TODO: this is only right if all regions have the same size, WORD_SIZE
        let idx = (addr + HALF_WORD) as usize;
        if let Some(word) = match region {
            RegionType::Ram => self.ram.get_mut(idx),
            RegionType::Sto => self.sto.get_mut(idx),
        } {
            *word = value;
            Ok(())
        } else {
            Err(format!("Address {} out of range!", addr))
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
                return Err(format!("PC is out of bounds: {}", self.pc));
            }
        } else {
            self.pc -= 1;
            if self.pc < -HALF_WORD {
                return Err(format!("PC is out of bounds: {}", self.pc));
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

fn char_to_digit(c: char) -> Option<i16> {
    match c {
        '.' => Some(0),
        'a'..='m' => Some((c as u8 - b'a' + 1) as i16),
        'n'..='z' => Some(c as i16 - b'n' as i16 - SIP_SIZE / 2),
        'A'..='M' => Some((c as u8 - b'A' + 1) as i16),
        'N'..='Z' => Some(c as i16 - b'N' as i16 - SIP_SIZE / 2),
        _ => None,
    }
}

fn digit_to_char(d: i16) -> Option<char> {
    match d {
        0 => Some('.'),
        1..=13 => Some((b'a' + (d as u8) - 1) as char),
        -13..=-1 => Some((b'n' + (d + SIP_SIZE / 2) as u8) as char),
        _ => None,
    }
}

fn i16_to_code(mut value: i16) -> String {
    let mut digits = Vec::new();

    while value != 0 {
        let mut rem = value % SIP_SIZE;
        value /= SIP_SIZE;
        if rem > SIP_SIZE / 2 {
            rem -= SIP_SIZE;
            value += 1;
        } else if rem < -SIP_SIZE / 2 {
            rem += SIP_SIZE;
            value -= 1;
        }
        digits.insert(0, rem);
    }

    let mut code = String::new();
    for d in digits {
        code.push(digit_to_char(d).unwrap_or('?'));
    }

    while code.len() < SIP_PER_WORD as usize {
        code.insert(0, '.');
    }

    code
}
fn code_to_i16(token: &str) -> Result<i16, String> {
    let mut value = 0i16;
    if token.chars().all(|c| c == '.' || c.is_ascii_alphabetic()) {
        for char in token.chars() {
            let d = char_to_digit(char).ok_or_else(|| format!("Invalid character: {}", char))?;
            value = value * SIP_SIZE + d;
        }
    } else {
        value = token
            .parse::<i16>()
            .map_err(|_| format!("Invalid number: {}", token))?
    }

    if value < -(HALF_WORD) || value > HALF_WORD {
        Err(format!("Value out of word range: {}", value))
    } else {
        Ok(value)
    }
}
