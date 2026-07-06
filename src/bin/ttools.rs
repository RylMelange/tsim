use std::collections::HashMap;
use std::{env, fs};
use ternary_tools::helper::*;

#[derive(Debug)]
struct Reference {
    output_index: i32,
    label: String,
    offset: i32,
}

struct Assembler {
    labels: HashMap<String, i32>,

    negative: Vec<String>,
    positive: Vec<String>,

    references: Vec<Reference>,

    pc: i32,
}

impl Assembler {
    fn new() -> Self {
        Self {
            labels: HashMap::new(),
            negative: vec!["...".to_string()],
            positive: vec!["...".to_string()],
            references: Vec::new(),
            pc: 1,
        }
    }

    fn emit(&mut self, text: String) {
        const PRINT: bool = false;
        if self.pc >= 0 {
            let index = self.pc as usize;
            if self.positive.len() <= index {
                self.positive.resize(index + 1, "...".to_string());
            }
            if PRINT {
                println!("writing {} to {}", text, index);
            }
            self.positive[index] = text;
            self.pc += 1;
        } else {
            let index = -self.pc as usize;
            if self.negative.len() <= index {
                self.negative.resize(index + 1, "...".to_string());
            }
            if PRINT {
                println!("writing {} to {}", text, -(index as i32));
            }
            self.negative[index] = text;
            self.pc -= 1;
        }
    }

    fn assemble(mut self, source: &str) -> Result<Vec<String>, String> {
        // Pass 1
        for line in source.lines() {
            let line = line.split(';').next().unwrap().trim();

            if line.is_empty() {
                continue;
            }

            for token in line.split_whitespace() {
                if token.chars().all(|c| c.is_ascii_uppercase()) {
                    self.emit(token.to_string());
                } else if token.starts_with('>') {
                    let label = token[1..].to_string();
                    self.labels.insert(label, self.pc);
                } else if token.starts_with(':') {
                    let label = token[1..].to_string();
                    let new_pc = if self.positive.len() > self.negative.len() {
                        -(self.negative.len() as i32)
                    } else {
                        self.positive.len() as i32
                    };
                    self.pc = new_pc;
                    self.labels.insert(label, self.pc);
                } else if token.starts_with('?') {
                    let label = token[1..].to_string();
                    if !self.labels.contains_key(&label) {
                        let new_label = self.positive.len().max(self.negative.len()) as i32;
                        self.pc = if self.pc > 0 { new_label } else { -new_label };
                        self.labels.insert(label.clone(), self.pc);
                    } else {
                        self.pc = *self.labels.get(&label).unwrap()
                    }
                } else if token.starts_with('!') {
                    let label = token[1..].to_string();
                    if !self.labels.contains_key(&label) {
                        let new_label = self.positive.len().max(self.negative.len()) as i32;
                        self.pc = if self.pc > 0 { new_label } else { -new_label };
                        self.labels.insert(label.clone(), -self.pc);
                    } else {
                        self.pc = -*self.labels.get(&label).unwrap()
                    }
                } else if token.starts_with('#') {
                    let literal = token[1..].to_string();
                    self.emit(literal.to_string());
                } else if token.starts_with('.') {
                    let parity = token.chars().nth(1).unwrap().to_string();
                    let sign = if parity == "-" { -1 } else { 1 };
                    let decimal = str::parse::<i32>(&token[2..]).unwrap();
                    let realsign = if self.pc < 0 { -sign } else { sign };
                    let addr = self.pc + realsign * decimal;
                    self.emit(i16_to_code(addr as i16));
                } else {
                    let (label, offset) = parse_reference(token)?;

                    let reference = Reference {
                        output_index: self.pc,
                        label,
                        offset,
                    };

                    self.emit("???".into());

                    self.references.push(reference);
                }
            }
        }

        // Pass 2
        for reference in &self.references {
            let addr = self
                .labels
                .get(&reference.label)
                .ok_or_else(|| format!("Unknown label {}", reference.label))?;
            let address = i16_to_code((addr + reference.offset) as i16);

            if reference.output_index > 0 {
                self.positive[reference.output_index as usize] = format!("{:03}", address);
            } else {
                self.negative[-reference.output_index as usize] = format!("{:03}", address);
            }
        }

        // let max_length = self.positive.len().max(self.negative.len());
        let max_length = 9842;
        self.positive.resize(max_length, "...".to_string());
        self.negative.resize(max_length, "...".to_string());
        self.negative.remove(0);

        // for x in 1..max_length {
        //     println!(
        //         "({:3})    {}: {:3}  -  {:3} :{}",
        //         x,
        //         i16_to_code(x as i16),
        //         self.positive.get(x).unwrap(),
        //         self.negative.get(x).unwrap(),
        //         i16_to_code(-(x as i16))
        //     );
        // }

        self.negative.reverse();
        self.negative.extend(self.positive);

        Ok(self.negative)
    }
}

fn parse_reference(token: &str) -> Result<(String, i32), String> {
    if let Some((name, off)) = token.split_once('+') {
        Ok((name.to_string(), off.parse().map_err(|_| "bad offset")?))
    } else if let Some((name, off)) = token.split_once('-') {
        Ok((
            name.to_string(),
            -off.parse::<i32>().map_err(|_| "bad offset")?,
        ))
    } else {
        Ok((token.to_string(), 0))
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: ttools <input.asm>");
        return;
    }

    let input = fs::read_to_string(&args[1]).expect("Couldn't read input file");

    let asm = Assembler::new();

    match asm.assemble(&input) {
        Ok(out) => {
            for x in out {
                println!("{x}");
            }
        }
        Err(e) => println!("{e}"),
    }
}
