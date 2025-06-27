use std::fmt::{Debug, Display};

pub type Integer = i32;
pub type BiggerInteger = i64;

const _: () = assert!(
    BiggerInteger::BITS > Integer::BITS,
    "BiggerInteger should be bigger than Integer"
);

const _: () = assert!(Integer::MIN < 0, "Integer should be signed");
const _: () = assert!(BiggerInteger::MIN < 0, "BiggerInteger should be signed");

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DigitInteger {
    value: Integer,
    digits: u8,
}

impl DigitInteger {
    pub const MAXIMUM_DIGITS: usize = Integer::MAX.ilog10() as usize - 1;

    pub fn new(value: Integer, digits: u8) -> Result<Self, AssignIntegerError> {
        if digits <= Self::MAXIMUM_DIGITS as u8 {
            Self::check_value(value, digits)?;

            Ok(Self { value, digits })
        } else {
            Err(AssignIntegerError::NumDigitsNotSupported)
        }
    }

    #[must_use]
    pub fn zero(digits: u8) -> Self {
        Self { value: 0, digits }
    }

    pub fn try_set(&mut self, value: Integer) -> Result<(), AssignIntegerError> {
        self.value = Self::check_value(value, self.digits)?;
        Ok(())
    }

    #[must_use]
    pub fn get(&self) -> Integer {
        self.value
    }

    #[must_use]
    pub fn get_bigger(&self) -> BiggerInteger {
        self.value as BiggerInteger
    }

    fn check_value(value: Integer, digits: u8) -> Result<Integer, AssignIntegerError> {
        let digit_range = Self::range_of_digits(digits);

        if value > digit_range {
            Err(AssignIntegerError::ValueTooBig {
                got: value,
                maximum: digit_range,
            })
        } else if value < -digit_range {
            Err(AssignIntegerError::ValueTooSmall {
                got: value,
                minimum: -digit_range,
            })
        } else {
            Ok(value)
        }
    }

    #[must_use]
    pub fn maximum(&self) -> Integer {
        Self::range_of_digits(self.digits)
    }

    #[must_use]
    pub fn minimum(&self) -> Integer {
        -self.maximum()
    }

    #[must_use]
    fn range_of_digits(digits: u8) -> Integer {
        const DIGIT_COMBINATIONS: [Integer; DigitInteger::MAXIMUM_DIGITS + 1] = {
            let mut result = [0; DigitInteger::MAXIMUM_DIGITS + 1];
            let mut acc = 0;

            let mut i = 0;

            while i < result.len() {
                acc *= 10;
                acc += 9;

                result[i] = acc;

                i += 1;
            }

            result
        };

        DIGIT_COMBINATIONS[digits as usize]
    }
}

impl Display for DigitInteger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl Debug for DigitInteger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let digits = self.value.abs().to_string();

        if self.value < 0 {
            write!(f, "-")?;
        }

        for _ in digits.len()..self.digits as usize + 1 {
            write!(f, "0")?;
        }

        write!(f, "{digits}")
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AssignIntegerError {
    ValueTooBig {
        got: Integer,
        maximum: Integer,
    },
    ValueTooSmall {
        got: Integer,
        minimum: Integer,
    },

    ValueMuchTooBig {
        got: BiggerInteger,
        maximum: Integer,
    },
    ValueMuchTooSmall {
        got: BiggerInteger,
        minimum: Integer,
    },
    NumDigitsNotSupported,
}
