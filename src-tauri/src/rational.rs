use std::cmp::Ordering;
use std::fmt;
use std::ops;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rational {
    pub num: i128,
    pub den: i128,
}

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

impl Rational {
    pub fn new(num: i128, den: i128) -> Self {
        if den == 0 {
            panic!("denominator cannot be zero");
        }
        let mut r = Rational { num, den };
        r.normalize();
        r
    }

    pub fn from_int(n: i128) -> Self {
        Rational { num: n, den: 1 }
    }

    pub fn zero() -> Self {
        Rational { num: 0, den: 1 }
    }

    pub fn one() -> Self {
        Rational { num: 1, den: 1 }
    }

    pub fn normalize(&mut self) {
        if self.num == 0 {
            self.den = 1;
            return;
        }
        let g = gcd(self.num.abs(), self.den);
        self.num /= g;
        self.den /= g;
        if self.den < 0 {
            self.num = -self.num;
            self.den = -self.den;
        }
    }

    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    pub fn is_one(&self) -> bool {
        self.num == 1 && self.den == 1
    }

    pub fn is_negative(&self) -> bool {
        self.num < 0
    }

    pub fn is_integer(&self) -> bool {
        self.den == 1
    }

    pub fn abs(&self) -> Self {
        Rational::new(self.num.abs(), self.den)
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            let num = self.num;
            let den = self.den;
            if num < 0 {
                write!(f, "-{}/{}", -num, den)
            } else {
                write!(f, "{}/{}", num, den)
            }
        }
    }
}

impl ops::Add for Rational {
    type Output = Rational;
    fn add(self, other: Rational) -> Rational {
        let num = self.num * other.den + other.num * self.den;
        let den = self.den * other.den;
        Rational::new(num, den)
    }
}

impl ops::Sub for Rational {
    type Output = Rational;
    fn sub(self, other: Rational) -> Rational {
        let num = self.num * other.den - other.num * self.den;
        let den = self.den * other.den;
        Rational::new(num, den)
    }
}

impl ops::Mul for Rational {
    type Output = Rational;
    fn mul(self, other: Rational) -> Rational {
        Rational::new(self.num * other.num, self.den * other.den)
    }
}

impl ops::Div for Rational {
    type Output = Rational;
    fn div(self, other: Rational) -> Rational {
        Rational::new(self.num * other.den, self.den * other.num)
    }
}

impl ops::Neg for Rational {
    type Output = Rational;
    fn neg(self) -> Rational {
        Rational::new(-self.num, self.den)
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.num * other.den).cmp(&(other.num * self.den))
    }
}

impl std::iter::Sum for Rational {
    fn sum<I: Iterator<Item = Rational>>(iter: I) -> Self {
        iter.fold(Rational::zero(), |a, b| a + b)
    }
}
