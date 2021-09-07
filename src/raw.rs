use bigdecimal::BigDecimal;

use std::str::FromStr;
use std::num::ParseIntError;
use std::ops::{Add, Sub, Div, Mul};

pub const RAW_MNANO_STR : &str = "1000000000000000000000000000000.0";

pub struct Raw {
    pub raw: u128
}

impl Raw {

    pub fn new(raw: u128) -> Raw {
        Raw { raw }
    }

    pub fn from_mnano(nano : BigDecimal) -> Raw {
        let dec =  BigDecimal::from_str(RAW_MNANO_STR).unwrap();
        let mut raw_d = nano * dec;
        raw_d = raw_d.with_scale(0);
        // TODO: from_u128 seems broken so we're using strings.
        let raw = u128::from_str(raw_d.to_string().as_str()).unwrap();
        Raw{raw}
    }

    // https://docs.nano.org/protocol-design/distribution-and-units/#unit-dividers
    pub fn to_mnano(&self) -> BigDecimal {
        let base_d = BigDecimal::from_str(RAW_MNANO_STR).unwrap();
        // TODO: from_u128 seems broken so we're using strings.
        let raw_d = BigDecimal::from_str(self.raw.to_string().as_str()).unwrap(); 
        raw_d/base_d
    }
}

impl FromStr for Raw {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(u128::from_str(s)?))
    }
}

impl ToString for Raw {
    fn to_string(&self) -> String {
        self.raw.to_string()
    }
}

impl Add for Raw {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.raw + rhs.raw)
    }
}

impl Sub for Raw {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.raw - rhs.raw)
    }
}

impl Mul for Raw {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.raw * rhs.raw)
    }
}

impl Div for Raw {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.raw / rhs.raw)
    }
}


#[cfg(test)]
mod tests {

    use super::*;
    use bigdecimal::FromPrimitive;

    #[test]
    fn valid_raw_to_mnano_conversion() {
        let raw = Raw::new(95_000_000_000_000_000_000_000_000_000);
        assert_eq!(
            raw.to_mnano(),
            BigDecimal::from_f64(0.095).unwrap()
        );
    }

    #[test]
    fn valid_mnano_to_raw_conversion() {
        let raw = Raw::new(95_000_000_000_000_000_000_000_000_000);
        let dec = BigDecimal::from_f64(0.095).unwrap();
        assert_eq!(
            Raw::from_mnano(dec).raw,
            raw.raw
        );
    }
}