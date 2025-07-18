use std::{
    io::BufRead,
    time::{Duration, Instant},
};

use computer::Computer;

use crate::simulation::program::Program;

pub mod argument;
pub mod computer;
pub mod instruction;
pub mod integer;
pub mod program;

pub fn interactively_run(computer: &mut Computer, program: &Program) {
    let mut skip_ticks = 0;
    let mut repeat_ns = None;
    let mut print_execution = true;
    let mut timer_start: Option<Instant> = None;

    loop {
        let instruction = computer.instruction;

        let modified = computer.step_cycle(program);
        let block_time = computer.block_time;

        if computer.block_time > 0 {
            computer.step_instruction(program);
        }

        if let Some(interrupt) = computer.interrupt {
            if let Some(instruction) = program.instructions.get(instruction as usize) {
                print!(
                    "\nHalted on line {line} ({instruction}):",
                    line = instruction.line
                );
            }

            if let Some(start) = timer_start {
                print!("\n(took {:?})", start.elapsed());
            }

            println!(
                "\n{interrupt:?}\n{registers}\n\nRuntime: {runtime}\nEnergy used: {energy}",
                registers = computer.registers,
                runtime = computer.runtime,
                energy = computer.energy_used,
            );

            break;
        }

        if computer.tick_complete {
            if print_execution {
                if block_time == 0 {
                    println!("completed tick");
                } else {
                    println!("completed {} ticks...", block_time + 1);
                }
            }

            if skip_ticks > 0 {
                skip_ticks -= 1;

                if skip_ticks == 0 {
                    print_execution = true;

                    if let Some(start) = timer_start {
                        println!("(took {:?})", start.elapsed());

                        timer_start = None;
                    }
                }
            }
        }

        if print_execution {
            if let Some(instruction) = program.instructions.get(instruction as usize) {
                print!("Line {line} ({instruction})", line = instruction.line);
            } else {
                print!("Instruction {instruction}");
            }

            println!(":");

            if modified {
                println!("{}", computer.registers);
            }
        }

        if skip_ticks != 0 {
            if let Some(repeat_ns) = repeat_ns {
                if repeat_ns > 0 {
                    std::thread::sleep(Duration::from_nanos(repeat_ns));
                }
            }
        } else {
            let string = &mut String::new();

            std::io::stdin().lock().read_line(string).unwrap();

            let mut tokens = string.split_whitespace();

            if string.is_empty() {
                // When not running interactively, add the missing newline
                println!();
            } else if let Some(input) = tokens.next() {
                if ["e", "end"].contains(&input) {
                    skip_ticks = u64::MAX;
                } else if let Ok(input) = input.parse() {
                    skip_ticks = input;
                }

                if tokens.next() == Some("delay") {
                    if let Some(Some(input)) = tokens.next().map(|input| input.parse::<u64>().ok())
                    {
                        let conversion_factor = match tokens.next() {
                            Some("ns" | "nanosecond" | "nanoseconds") => 0,
                            Some("μs" | "us" | "microsecond" | "microseconds") => 3,
                            Some("ms" | "millisecond" | "milliseconds") => 6,
                            Some("s" | "second" | "seconds") | None => 9,
                            _ => 9,
                        };

                        repeat_ns = Some(input * 10u64.pow(conversion_factor));
                    } else {
                        repeat_ns = None;
                    }
                }

                if repeat_ns.is_none() {
                    timer_start = Some(Instant::now());

                    if skip_ticks > 0 {
                        print_execution = false;
                    }
                }
            }
        }
    }
}
