mod process;
use process::{CustomProcess, ProcessSpawnError, ShellProcess};

use clap::Parser;
use std::env;
use std::io::{self, Write};
use std::path::Path;

use colored::Colorize;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    debug_info: bool,
}

#[derive(Debug)]
struct Request {
    name: String,
    args: Vec<String>,
}

macro_rules! output {
    () => {
        anstream::println!("{}", "[vshell]".blue().bold());
    };

    ($fmt:expr $(, $arg:expr)* $(,)?) => {
        anstream::println!(
            concat!("{}", " ", $fmt),
            "[vshell]".blue().bold(),
            $($arg),*
        );
    };
}

fn cd_command(args: Vec<&str>) -> i32 {
    let target_dir = if args.is_empty() {
        match env::var("HOME") {
            Ok(home) => home,
            Err(_) => return 1,
        }
    } else {
        args[0].to_string()
    };

    match env::set_current_dir(Path::new(&target_dir)) {
        Ok(_) => 0,
        Err(e) => {
            anstream::eprintln!("{}", format!("cd: {}", e).red());
            1
        }
    }
}

type CommandHandler = fn(Vec<&str>) -> i32;
type CommandDef = (CommandHandler, &'static str);

const COMMANDS: [CommandDef; 1] = [(cd_command, "cd")];

fn slice_request(req: &str) -> Vec<String> {
    req.split_whitespace().map(String::from).collect()
}

fn parse_request(req: &str, debug: bool) -> Request {
    let sliced = slice_request(req);

    let mut result: Request = Request {
        name: String::from(""),
        args: vec![],
    };

    for (i, part) in sliced.iter().enumerate() {
        if i == 0 {
            // name
            result.name = part.clone();
        } else {
            // argument ?
            result.args.push(part.clone());
        }
    }

    if debug {
        output!("{}", format!("Parsing finished {:?}", result));
    }

    result
}

fn run_request(req: Request, debug: bool) -> i32 {
    if req.name.is_empty() {
        return 0;
    }

    let mut result = 0;

    let mut builtin_found = false;

    // Check for built-in command
    for (func, name) in COMMANDS.iter() {
        if req.name == *name {
            let refs: Vec<&str> = req.args.iter().map(|s| s.as_str()).collect();

            result = func(refs);
            builtin_found = true;
            break;
        }
    }

    if !builtin_found {
        // External command
        match CustomProcess::spawn(&req.name, &req.args) {
            Ok(mut proc) => {
                if debug {
                    output!("{}", format!("Running. PID: {}", proc.id()));
                }

                match proc.wait() {
                    Ok(code) => {
                        if debug {
                            output!("{}", format!("Program exited with code: {}", code));
                        }

                        result = code;
                    }

                    Err(error) => {
                        if debug {
                            output!("{}", format!("Wait error: {}", error));
                        }

                        result = 1;
                    }
                }
            }

            Err(ProcessSpawnError::NotFound { program }) => {
                output!(
                    "{}",
                    format!("Command not found: {}", program.white().bold()).red()
                );
                result = 127;
            }

            Err(ProcessSpawnError::PermissionDenied { program }) => {
                output!(
                    "{}",
                    format!("Permission denied: {}", program.white().bold()).red()
                );
                result = 126;
            }

            Err(error) => {
                output!("{}", format!("Failed to execute '{}': {}", req.name, error));
                result = 1;
            }
        }
    }

    result
}

fn main() {
    let args = Args::parse();
    let debug = args.debug_info;

    if debug {
        output!("Debug is on!");
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        let mut input = String::new();

        anstream::print!("{}: ", whoami::username().green());

        stdout.flush().expect("Flush failed!");
        stdin.read_line(&mut input).expect("Read line failed!");

        // remove \n\r\t at start\end
        let input = input.trim();

        if input == "exit" {
            break;
        }

        let parse_res = parse_request(input, debug);
        let _run_res = run_request(parse_res, debug);
    }
}
