use crate::machine::machine::Machine;
use clap::Parser;
use raylib::prelude::*;
use std::path::PathBuf;
mod machine;

#[derive(Parser, Debug)]
#[command(name = "tsim", version, about = "A binary VM", long_about = None)]
struct Args {
    /// Load a state file
    #[arg(short, long)]
    dump: Option<PathBuf>,
    /// Load a RAM file
    #[arg(short, long)]
    ram: Option<PathBuf>,
    /// Load a STO file
    #[arg(short, long)]
    sto: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let mut machine = Machine::default();

    if let Some(path) = args.ram {
        match machine.load_state(&path) {
            Ok(_) => {
                println!("Loaded RAM!");
            }
            Err(e) => {
                eprintln!("Error loading {}: {}", path.display(), e);
            }
        }
    };
    if let Some(path) = args.sto {
        match machine.load_state(&path) {
            Ok(_) => {
                println!("Loaded STO!");
            }
            Err(e) => {
                eprintln!("Error loading {}: {}", path.display(), e);
            }
        }
    };
    if let Some(path) = args.dump {
        match machine.load_state(&path) {
            Ok(_) => {
                println!("Loaded state!");
            }
            Err(e) => {
                eprintln!("Error loading {}: {}", path.display(), e);
            }
        }
    };

    let (mut rl, thread) = raylib::init().size(1200, 800).title("Ternary VM").build();

    while !rl.window_should_close() {
        let d = &mut rl.begin_drawing(&thread);
        d.clear_background(Color::DARKSLATEBLUE);
    }
}

// fn handle_command(machine: &mut Machine, input: &str) -> Result<(), String> {
//     let mut args: Vec<&str> = input.trim().split_whitespace().collect();
//     if args.is_empty() {
//         return Ok(());
//     }
//
//     let cmd = args.remove(0).to_lowercase();
//     match cmd.as_str() {
//         "q" => std::process::exit(0),
//         "state" => {
//             if args.is_empty() {
//                 machine.print_state(false);
//             } else if args[0].eq_ignore_ascii_case("all") {
//                 machine.print_state(true);
//             } else {
//                 return Err("Usage: state [all]".to_string());
//             }
//             Ok(())
//         }
//         "test" => {
//             let num = code_to_i16(args[0])?;
//             println!("{}", num);
//             Ok(())
//         }
//         "step" | "\'" => {
//             machine.paused = false;
//             machine.step()?;
//             machine.print_state(false);
//             Ok(())
//         }
//         "run" | "/" => {
//             machine.paused = false;
//             match args.len() {
//                 0 => machine.run(None)?,
//                 1 => machine.run(Some(
//                     args[0]
//                         .parse()
//                         .map_err(|_| format!("Invalid number: {}", args[0]))?,
//                 ))?,
//                 _ => return Err("Usage: run [steps]".to_string()),
//             }
//             machine.print_state(false);
//             Ok(())
//         }
//         "dump" => {
//             if args.is_empty() {
//                 return Err("Usage: dump <ram|sto> [start end]".to_string());
//             }
//             let region = match args[0].to_lowercase().as_str() {
//                 "ram" => RegionType::Ram,
//                 "sto" => RegionType::Sto,
//                 _ => {
//                     return Err(format!("Error: expected RAM or STO"));
//                 }
//             };
//             let (start, end) = match args.len() {
//                 // TODO: this is only right if all regions have the same size, WORD_SIZE
//                 1 => (-(HALF_WORD), HALF_WORD),
//                 2 => {
//                     let s = code_to_i16(args[1])?;
//                     (s, s)
//                 }
//                 3 => {
//                     let s = code_to_i16(args[1])?;
//                     let e = code_to_i16(args[2])?;
//                     if s <= e { (s, e) } else { (e, s) }
//                 }
//                 _ => return Err("Usage: dump <ram|sto> [start end]".to_string()),
//             };
//             machine.dump_region(region, start, end)
//         }
//         "pc" => {
//             if args.len() != 1 {
//                 return Err("Usage: pc <value>".to_string());
//             }
//             machine.pc = code_to_i16(args[0])?;
//             machine.paused = false;
//             Ok(())
//         }
//         "a" => {
//             if args.len() != 1 {
//                 return Err("Usage: a <value>".to_string());
//             }
//             machine.a = code_to_i16(args[0])?;
//             Ok(())
//         }
//         "b" => {
//             if args.len() != 1 {
//                 return Err("Usage: b <value>".to_string());
//             }
//             machine.b = code_to_i16(args[0])?;
//             Ok(())
//         }
//         "sar" => {
//             if args.len() != 1 {
//                 return Err("Usage: sar <value>".to_string());
//             }
//             machine.sar = code_to_i16(args[0])?;
//             Ok(())
//         }
//         "write" => {
//             // TODO: maybe accept assembly codes as well
//             if args.len() < 2 {
//                 return Err(format!("Usage: write <ram|sto> <addr>"));
//             }
//
//             let region = match args[0].to_lowercase().as_str() {
//                 "ram" => RegionType::Ram,
//                 "sto" => RegionType::Sto,
//                 _ => {
//                     return Err(format!("Error: expected RAM or STO"));
//                 }
//             };
//
//             let addr = code_to_i16(args[1])?;
//
//             // TODO: add some way to abort
//             print!("Enter data to write: ");
//
//             let _ = io::stdout().flush();
//             let mut buf = String::new();
//             io::stdin().read_line(&mut buf).map_err(|e| e.to_string())?;
//
//             let t = buf.trim();
//             if t.is_empty() {
//                 println!("Cancelled.");
//                 return Ok(());
//             }
//             let data = t.to_string();
//
//             machine.write_stream(region, addr, &data)
//         }
//         "load" => {
//             if args.len() == 1 {
//                 machine.load_state(PathBuf::from(&args[0]).as_path())
//             } else {
//                 return Err(format!("Usage: load <file>"));
//             }
//         }
//         "save" => {
//             let mut args = args.into_iter();
//             let path = args
//                 .next()
//                 .ok_or(format!("Usage: save <file> [all|sto|ram]"))?;
//             let mut states = States::empty();
//
//             if args.len() == 0 {
//                 states = States::ALL;
//             } else {
//                 while let Some(arg) = args.next() {
//                     if arg.eq_ignore_ascii_case("all") {
//                         states = states | States::ALL;
//                     } else if arg.eq_ignore_ascii_case("sto") {
//                         states = states | States::STO;
//                     } else if arg.eq_ignore_ascii_case("ram") {
//                         states = states | States::RAM;
//                     } else {
//                         return Err(format!("Usage: save <file> [all|sto|ram]"));
//                     }
//                 }
//             }
//
//             machine.save_state(states, PathBuf::from(path).as_path())
//         }
//         _ => {
//             println!("Unknown command: {}", cmd);
//             println!("TODO: write available commands here");
//             Ok(())
//         }
//     }
// }
