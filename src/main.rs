use clap::Parser;
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
    paused: bool,
    ram: Vec<i16>,
    sto: Vec<i16>,
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

        if !trimmed.is_empty() {
            handle_command(machine, trimmed);
        }
    }
}

fn handle_command(machine: &mut Machine, input: &str) {
    let mut parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    let cmd = parts.remove(0).to_lowercase();
    match cmd.as_str() {
        "load" => {
            if parts.len() == 1 {
                match machine.load_state(PathBuf::from(&parts[0]).as_path()) {
                    Ok(_) => println!("Loaded from {}!", parts[0]),
                    Err(e) => eprintln!("Error loading from {}: {}", parts[0], e),
                }
            }
        }
        "save" => {
            if parts.len() == 1 {
                match machine.save_state(PathBuf::from(&parts[0]).as_path()) {
                    Ok(_) => println!("Saved to {}!", parts[0]),
                    Err(e) => eprintln!("Error writing to {}: {}", parts[0], e),
                }
            } else {
                println!("Usage: save <file>");
            }
        }
        "q" => std::process::exit(0),
        _ => {
            println!("Unknown command: {}", cmd);
            println!("TODO: write available commands here")
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            pc: 0,
            a: 0,
            b: 0,
            paused: false,
            ram: vec![0; RAM_SIZE],
            sto: vec![0; STO_SIZE],
        }
    }
}

impl Machine {
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
}

fn char_to_digit(c: char) -> Option<i16> {
    match c {
        '.' => Some(0),
        'a'..='m' => Some((c as u8 - b'a' + 1) as i16),
        'n'..='z' => Some((c as u8 - b'n' - SIP_SIZE as u8) as i16),
        'A'..='M' => Some((c as u8 - b'A' + 1) as i16),
        'N'..='Z' => Some((c as u8 - b'N' - SIP_SIZE as u8) as i16),
        _ => None,
    }
}

fn digit_to_char(d: i16) -> Option<char> {
    match d {
        0 => Some('.'),
        1..=13 => Some((b'a' + (d as u8) - 1) as char),
        -13..=-1 => Some((b'n' + (d + SIP_SIZE) as u8) as char),
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
