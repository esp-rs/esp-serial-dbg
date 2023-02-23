use clap::{Parser, Subcommand};
use parse_int::parse;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::time::Duration;
use std::{io, thread};

mod gdb;

use espdbg::*;

#[derive(Debug, Parser)] // requires `derive` feature
#[clap(name = "espdbg")]
#[clap(about = "esp-serial-dbg CLI", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(arg_required_else_help = true, about = "Developer's CLI")]
    Cli { chip: Chip, port: Option<String> },
    #[clap(arg_required_else_help = true, about = "GDB Server")]
    Gdb { chip: Chip, port: Option<String> },
}

#[derive(Debug, Clone, Copy)]
enum Flavor {
    Cli,
    Gdb,
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    let (flavor, chip, port) = match args.command {
        Commands::Cli { chip, port } => (Flavor::Cli, chip, port),
        Commands::Gdb { chip, port } => (Flavor::Gdb, chip, port),
    };

    let port_to_use = if port.is_none() {
        let ports = serialport::available_ports().expect("No ports found!");
        if ports.len() > 1 {
            println!("More than one serial port found.");
            return;
        } else {
            ports[0].port_name.clone()
        }
    } else {
        port.unwrap()
    };

    let port = serialport::new(port_to_use, 115_200)
        .timeout(Duration::from_millis(1))
        .open()
        .expect("Failed to open port");

    let dbg = SerialDebugConnection::new(chip, port);

    match flavor {
        Flavor::Cli => run_cli(dbg),
        Flavor::Gdb => run_gdb(dbg),
    }
}

fn run_gdb(dbg: SerialDebugConnection) {
    thread::scope(|s| {
        s.spawn(|| loop {
            match dbg.chip {
                Chip::Esp32C3 | Chip::Esp32C2 => {
                    gdb::gdb_main::<gdb::riscv_esp32c3::RiscvRegisters>(&dbg).unwrap()
                }
                Chip::Esp32 => {
                    gdb::gdb_main::<gdb::xtensa_esp32::XtensaEsp32Registers>(&dbg).unwrap()
                }
                Chip::Esp32S2 => {
                    gdb::gdb_main::<gdb::xtensa_esp32s2::XtensaEsp32S2Registers>(&dbg).unwrap()
                }
                Chip::Esp32S3 => {
                    gdb::gdb_main::<gdb::xtensa_esp32s3::XtensaEsp32S3Registers>(&dbg).unwrap()
                }
            }
        });

        s.spawn(|| dbg.start());

        let response = dbg.hello(std::time::Duration::from_millis(2000));
        if let Err(_) = response {
            dbg.reset_target();
        }

        // TODO graceful shutdown on ctrl-c
        // dbg.shutdown();
        // std::process::exit(0);
    });
}

fn run_cli(dbg: SerialDebugConnection) {
    thread::scope(|s| {
        s.spawn(|| dbg.start());

        let response = dbg.hello(std::time::Duration::from_millis(2000));
        if let Err(_) = response {
            dbg.reset_target();
        }

        let mut input_mode = false;
        let mut rl = Editor::<()>::new().unwrap();
        loop {
            if !input_mode {
                let mut buf = String::new();
                if let Ok(_n) = io::stdin().read_line(&mut buf) {
                    input_mode = true;
                    *dbg.muted.lock().unwrap() = true;
                }
            } else {
                if let Some(msg) = dbg.pending_message() {
                    println!("{:x?}", msg);
                }

                if !*dbg.wait_response.lock().unwrap() {
                    let readline = rl.readline(">> ");
                    match readline {
                        Ok(line) => {
                            let line = line.trim();
                            let parts: Vec<&str> = line.split(" ").collect();

                            match parts[0] {
                                "set-breakpoint" => {
                                    dbg.set_breakpoint(
                                        parse::<u32>(parts[1]).unwrap(),
                                        parse::<u8>(parts[2]).unwrap(),
                                    );
                                }
                                "clear-breakpoint" => {
                                    dbg.clear_breakpoint(parse::<u8>(parts[1]).unwrap());
                                }
                                "break" => {
                                    dbg.break_execution();
                                }
                                "set-watchpoint" => {}
                                "clear-watchpoint" => {}
                                "read-memory" => {
                                    let data = dbg.read_memory(
                                        parse::<u32>(parts[1]).unwrap(),
                                        parse::<u32>(parts[2]).unwrap(),
                                    );
                                    hexdump::hexdump(&data);
                                    //*dbg.wait_response.lock().unwrap() = true;
                                }
                                "write-memory" => {
                                    let mut data = Vec::new();
                                    for i in 2..parts.len() {
                                        data.push(parse::<u8>(parts[i]).unwrap());
                                    }
                                    dbg.write_memory(parse::<u32>(parts[1]).unwrap(), data);
                                }
                                "c" => {
                                    dbg.resume();
                                }
                                "q" => {
                                    break;
                                }
                                "" => {
                                    println!();
                                    *dbg.muted.lock().unwrap() = false;
                                    input_mode = false;
                                    continue;
                                }
                                _ => {
                                    println!("Unknown command {}", parts[0]);
                                }
                            }

                            rl.add_history_entry(line);
                        }
                        Err(ReadlineError::Interrupted) => {
                            break;
                        }
                        Err(ReadlineError::Eof) => {
                            break;
                        }
                        Err(err) => {
                            println!("Error: {:?}", err);
                            break;
                        }
                    }
                }
            }
        }

        dbg.shutdown();
        std::process::exit(0);
    });
}
