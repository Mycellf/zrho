use std::io::BufRead;

use crate::simulation::computer::Computer;

pub mod argument;
pub mod computer;
pub mod instruction;
pub mod integer;
pub mod program;

pub fn interactively_run(mut computer: Computer) {
    let mut skip_ticks = 0;

    loop {
        let instruction = computer.instruction;

        let modified = computer.step_cycle();

        if let Some(interrupt) = computer.interrupt {
            println!(
                "\n{:?}\n{}\n\nRuntime: {}",
                interrupt, computer.registers, computer.runtime,
            );
            break;
        }

        if skip_ticks == 0 && computer.block_time == 0 {
            print!("Instruction {:?}", instruction);

            if let Some(instruction) = computer
                .loaded_program
                .instructions
                .get(instruction as usize)
            {
                print!(" ({instruction})");
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
