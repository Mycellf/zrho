use std::io::BufRead;

use computer::Computer;

use crate::simulation::program::Program;

pub mod argument;
pub mod computer;
pub mod instruction;
pub mod integer;
pub mod program;

pub fn interactively_run(computer: &mut Computer, program: &Program) {
    let mut skip_ticks = 0;

    loop {
        let instruction = computer.instruction;

        let modified = computer.step_cycle(&program);

        if let Some(interrupt) = computer.interrupt {
            if let Some(instruction) = program.instructions.get(instruction as usize) {
                print!(
                    "\nHalted on line {line} ({instruction}):",
                    line = instruction.line
                );
            }

            println!(
                "\n{interrupt:?}\n{registers}\n\nRuntime: {runtime}\nEnergy used: {energy}",
                registers = computer.registers,
                runtime = computer.runtime,
                energy = computer.energy_used,
            );
            break;
        }

        if skip_ticks == 0 && computer.block_time == 0 {
            if let Some(instruction) = program.instructions.get(instruction as usize) {
                print!("Line {line} ({instruction})", line = instruction.line);
            } else {
                print!("Instruction {}", instruction);
            }

            println!(":");

            if modified {
                println!("{}", computer.registers);
            }
        }

        if computer.tick_complete {
            if skip_ticks == 0 {
                if computer.block_time == 0 {
                    println!("completed tick");
                } else {
                    println!(
                        "waiting {} tick{}...",
                        computer.block_time,
                        if computer.block_time == 1 { "" } else { "s" },
                    );
                }
            } else {
                skip_ticks -= 1;
            }
        }

        if skip_ticks == 0 && computer.block_time == 0 {
            let string = &mut String::new();

            std::io::stdin().lock().read_line(string).unwrap();

            if string.is_empty() {
                // When not running interactively, add the missing newline
                println!();
            } else if ["e", "end"].contains(&string.trim()) {
                skip_ticks = u64::MAX;
            } else if let Ok(input) = string.trim().parse::<u64>() {
                skip_ticks = input;
            }
        }
    }
}
