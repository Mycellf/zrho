use crate::{
    computer::{Computer, Program, Register, RegisterSet, RegisterValues},
    integer::DigitInteger,
};

pub mod integer;
// pub mod zrho_program;
pub mod computer;
pub mod instruction;

fn main() {
    let mut computer = Computer::new(
        Program::new_empty("Test Program".to_owned()),
        RegisterSet::new()
            .with_register(
                computer::register_with_name('X').unwrap(),
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(4)),
                    indexes_array: None,
                    read_time: 0,
                    read_block: 0,
                    write_time: 0,
                    write_block: 0,
                },
            )
            .with_register(
                computer::register_with_name('Y').unwrap(),
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(4)),
                    indexes_array: None,
                    read_time: 0,
                    read_block: 0,
                    write_time: 0,
                    write_block: 0,
                },
            )
            .with_register(
                computer::register_with_name('I').unwrap(),
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(4)),
                    indexes_array: Some(computer::register_with_name('D').unwrap()),
                    read_time: 0,
                    read_block: 0,
                    write_time: 0,
                    write_block: 0,
                },
            )
            .with_register(
                computer::register_with_name('D').unwrap(),
                Register {
                    values: RegisterValues::Vector {
                        values: Box::new([DigitInteger::zero(4); 100]),
                        index: 0,
                    },
                    indexes_array: None,
                    read_time: 0,
                    read_block: 0,
                    write_time: 0,
                    write_block: 0,
                },
            ),
    );

    computer
        .registers
        .get_mut(computer::register_with_name('X').unwrap())
        .unwrap()
        .value_mut()
        .unwrap()
        .try_set(10)
        .unwrap();
}
