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
    pub const DUMMY: Self = Self {
        value: 0,
        digits: 0,
    };

    pub const fn new(value: Integer, digits: u8) -> Result<Self, AssignIntegerError> {
        if digits <= Self::MAXIMUM_DIGITS as u8 {
            match Self::check_value(value, digits) {
                Ok(value) => Ok(Self { value, digits }),
                Err(error) => Err(error),
            }
        } else {
            Err(AssignIntegerError::NumDigitsNotSupported)
        }
    }

    pub const fn try_set(&mut self, value: Integer) -> Result<(), AssignIntegerError> {
        match Self::check_value(value, self.digits) {
            Ok(value) => {
                self.value = value;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    pub const fn is_valid(&self, value: Integer) -> Result<(), AssignIntegerError> {
        match Self::check_value(value, self.digits) {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
    }

    #[must_use]
    pub const fn get(&self) -> Integer {
        self.value
    }

    #[must_use]
    pub const fn get_bigger(&self) -> BiggerInteger {
        self.value as BiggerInteger
    }

    const fn check_value(value: Integer, digits: u8) -> Result<Integer, AssignIntegerError> {
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
    pub const fn range_of_digits(digits: u8) -> Integer {
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

impl Display for AssignIntegerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignIntegerError::ValueTooBig { got, maximum } => {
                format_overflow_error(f, got, *maximum)
            }
            AssignIntegerError::ValueTooSmall { got, minimum } => {
                format_underflow_error(f, got, *minimum)
            }
            AssignIntegerError::ValueMuchTooBig { got, maximum } => {
                format_overflow_error(f, got, *maximum)
            }
            AssignIntegerError::ValueMuchTooSmall { got, minimum } => {
                format_underflow_error(f, got, *minimum)
            }
            AssignIntegerError::NumDigitsNotSupported => {
                write!(f, "Number of digits not supported")
            }
        }
    }
}

pub fn format_overflow_error(
    f: &mut std::fmt::Formatter<'_>,
    value: &impl Display,
    maximum: Integer,
) -> std::fmt::Result {
    write!(
        f,
        "\"{value}\" is too big for this machine (maximum: {maximum})"
    )
}

pub fn format_underflow_error(
    f: &mut std::fmt::Formatter<'_>,
    value: &impl Display,
    minimum: Integer,
) -> std::fmt::Result {
    write!(
        f,
        "\"{value}\" is too small for this machine (minimum: {minimum})"
    )
}
