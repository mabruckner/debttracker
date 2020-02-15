use failure::{format_err, Error, Fail};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money(i64);

impl Money {
  pub fn from_cents(cents: i64) -> Money {
    Money(cents)
  }

  pub fn from_dollars(dollars: i64) -> Money {
    Money(dollars * 100)
  }

  pub fn from_money_string(string: String) -> Result<Money, Error> {
    let clean_string = string.replace("$", "");
    let parts: Vec<&str> = clean_string.split(".").collect();

    if parts.len() == 1 {
      Ok(Money::from_dollars(parts[0].parse()?))
    } else if parts.len() == 2 {
      let dollars = Money::from_dollars(parts[0].parse()?);
      let cents = Money::from_cents(parts[1].parse()?);
      Ok(dollars + cents)
    } else {
      Err(format_err!(
        "Money amount not of form x.xx or x: {}",
        string
      ))
    }
  }

  pub fn zero() -> Money {
    Money(0)
  }

  pub fn to_cents(&self) -> i64 {
    self.0
  }
}

impl fmt::Display for Money {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let sign = match self.0 {
      x if x > 0 => "+",
      x if x < 0 => "-",
      _ => "",
    };
    let dollars = (self.0 / 100).abs();
    let cents = self.0.abs() % 100;
    write!(f, "{}${}.{:0>2}", sign, dollars, cents)
  }
}

impl ops::Add for Money {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    Money(self.0 + other.0)
  }
}

impl ops::Sub for Money {
  type Output = Self;
  fn sub(self, other: Self) -> Self {
    Money(self.0 - other.0)
  }
}

impl ops::Neg for Money {
  type Output = Self;
  fn neg(self) -> Self {
    Money(-self.0)
  }
}

impl Ord for Money {
  fn cmp(&self, other: &Self) -> Ordering {
    self.0.cmp(&other.0)
  }
}

impl PartialOrd for Money {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}
