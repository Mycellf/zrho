pub type Integer = i32;
pub type BiggerInteger = i64;

const _: () = assert!(
    BiggerInteger::BITS > Integer::BITS,
    "BiggerInteger should be bigger than Integer"
);

const _: () = assert!(Integer::MIN < 0, "Integer should be signed");
const _: () = assert!(BiggerInteger::MIN < 0, "BiggerInteger should be signed");

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DigitInteger {
    value: Integer,
    digits: u8,
}

impl DigitInteger {
    pub const MAXIMUM_DIGITS: usize = Integer::MAX.ilog10() as usize;

    pub fn new(value: Integer, digits: u8) -> Result<Self, AssignIntegerError> {
        if digits <= Self::MAXIMUM_DIGITS as u8 {
            Self::check_value(value, digits)?;

            Ok(Self { value, digits })
        } else {
            Err(AssignIntegerError::NumDigitsNotSupported)
        }
    }

    pub fn try_set(&mut self, value: Integer) -> Result<(), AssignIntegerError> {
        self.value = Self::check_value(value, self.digits)?;
        Ok(())
    }

    pub fn get(&self) -> Integer {
        self.value
    }

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

    pub fn maximum(&self) -> Integer {
        Self::range_of_digits(self.digits)
    }

    pub fn minimum(&self) -> Integer {
        -self.maximum()
    }

    fn range_of_digits(digits: u8) -> Integer {
        const DIGIT_COMBINATIONS: [Integer; DigitInteger::MAXIMUM_DIGITS] = {
            let mut result = [0; DigitInteger::MAXIMUM_DIGITS];
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
