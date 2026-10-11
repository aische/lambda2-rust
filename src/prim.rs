//! Primitive operations with JavaScript `Number` semantics.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    // nullary
    Get,
    Random,
    // unary
    Not,
    Log,
    Sqrt,
    Floor,
    Ceil,
    Round,
    Put,
    Alert,
    // binary
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl Op {
    pub fn name(self) -> &'static str {
        match self {
            Op::Get => "get",
            Op::Random => "random",
            Op::Not => "not",
            Op::Log => "log",
            Op::Sqrt => "sqrt",
            Op::Floor => "floor",
            Op::Ceil => "ceil",
            Op::Round => "round",
            Op::Put => "put",
            Op::Alert => "alert",
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
            Op::Div => "/",
            Op::Rem => "%",
            Op::Pow => "^",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
            Op::Eq => "==",
            Op::Ne => "<>",
        }
    }

    pub fn binary(token: &str) -> Option<Op> {
        Some(match token {
            "+" => Op::Add,
            "-" => Op::Sub,
            "*" => Op::Mul,
            "/" => Op::Div,
            "%" => Op::Rem,
            "^" => Op::Pow,
            "<" => Op::Lt,
            "<=" => Op::Le,
            ">" => Op::Gt,
            ">=" => Op::Ge,
            "==" => Op::Eq,
            "<>" => Op::Ne,
            _ => return None,
        })
    }

    /// Unary operators that are plain operators in every strategy (`put`/`alert` are handled
    /// separately by the parser because they are IO actions under lazy evaluation).
    pub fn unary(token: &str) -> Option<Op> {
        Some(match token {
            "not" => Op::Not,
            "log" => Op::Log,
            "sqrt" => Op::Sqrt,
            "floor" => Op::Floor,
            "ceil" => Op::Ceil,
            "round" => Op::Round,
            _ => return None,
        })
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

fn truthy(n: f64) -> bool {
    n != 0.0 && !n.is_nan()
}

fn bool_num(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

/// Pure unary operators (`put`/`alert` have effects and are handled by the machine).
pub fn op1(op: Op, n: f64) -> f64 {
    match op {
        Op::Not => bool_num(!truthy(n)),
        Op::Log => n.ln(),
        Op::Sqrt => n.sqrt(),
        Op::Floor => n.floor(),
        Op::Ceil => n.ceil(),
        Op::Round => js_round(n),
        _ => unreachable!("{op} is not a pure unary operator"),
    }
}

pub fn op2(op: Op, a: f64, b: f64) -> f64 {
    match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => a * b,
        Op::Div => a / b,
        // Rust's `%` on f64 is fmod, which (like JS) takes the sign of the dividend.
        Op::Rem => a % b,
        Op::Pow => js_pow(a, b),
        Op::Lt => bool_num(a < b),
        Op::Le => bool_num(a <= b),
        Op::Gt => bool_num(a > b),
        Op::Ge => bool_num(a >= b),
        Op::Eq => bool_num(a == b),
        Op::Ne => bool_num(a != b),
        _ => unreachable!("{op} is not a binary operator"),
    }
}

/// `Math.round`: rounds half-way cases towards +∞ (`Math.round(-2.5) == -2`).
pub fn js_round(x: f64) -> f64 {
    let f = x.floor();
    // `x - floor(x)` is exact, so this also gets 0.49999999999999994 right (unlike `floor(x + 0.5)`).
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    if r == 0.0 && x.is_sign_negative() { -0.0 } else { r }
}

/// `Math.pow`, which differs from `powf` for a NaN exponent and for `(±1) ^ ±∞`.
pub fn js_pow(a: f64, b: f64) -> f64 {
    if b.is_nan() || (a.abs() == 1.0 && b.is_infinite()) { f64::NAN } else { a.powf(b) }
}

/// `parseFloat`: the longest prefix that is a decimal literal (or `Infinity`), else NaN.
pub fn js_parse_float(s: &str) -> f64 {
    let s = s.trim_start();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    if s[i..].starts_with("Infinity") {
        let inf = f64::INFINITY;
        return if b.first() == Some(&b'-') { -inf } else { inf };
    }
    let digits = |i: &mut usize| {
        let start = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - start
    };
    let mut mantissa = digits(&mut i);
    if i < b.len() && b[i] == b'.' {
        let mut j = i + 1;
        let frac = digits(&mut j);
        if mantissa + frac > 0 {
            mantissa += frac;
            i = j;
        }
    }
    if mantissa == 0 {
        return f64::NAN;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        if digits(&mut j) > 0 {
            i = j;
        }
    }
    s[..i].parse().unwrap_or(f64::NAN)
}

/// `Number.prototype.toString()`.
pub fn format_js(n: f64) -> String {
    ryu_js::Buffer::new().format(n).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_like_js() {
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert_eq!(js_round(-0.4), 0.0);
        assert!(js_round(-0.4).is_sign_negative());
        assert_eq!(js_round(1e300), 1e300);
        assert!(js_round(f64::NAN).is_nan());
        assert_eq!(js_round(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    #[test]
    fn remainder_sign_follows_dividend() {
        assert_eq!(op2(Op::Rem, -7.0, 3.0), -1.0);
        assert_eq!(op2(Op::Rem, 7.0, -3.0), 1.0);
    }

    #[test]
    fn pow_like_js() {
        assert!(js_pow(1.0, f64::NAN).is_nan());
        assert!(js_pow(-1.0, f64::INFINITY).is_nan());
        assert_eq!(js_pow(f64::NAN, 0.0), 1.0);
        assert_eq!(js_pow(2.0, 10.0), 1024.0);
    }

    #[test]
    fn not_treats_nan_as_false() {
        assert_eq!(op1(Op::Not, f64::NAN), 1.0);
        assert_eq!(op1(Op::Not, 0.0), 1.0);
        assert_eq!(op1(Op::Not, 5.0), 0.0);
    }

    #[test]
    fn format_like_js() {
        assert_eq!(format_js(100.0), "100");
        assert_eq!(format_js(1e21), "1e+21");
        assert_eq!(format_js(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(format_js(-0.0), "0");
        assert_eq!(format_js(47217363074749560.0), "47217363074749560");
        assert_eq!(format_js(f64::NAN), "NaN");
        assert_eq!(format_js(f64::INFINITY), "Infinity");
        assert_eq!(format_js(-5.0), "-5");
        assert_eq!(format_js(1e-7), "1e-7");
    }

    #[test]
    fn parse_float_like_js() {
        assert_eq!(js_parse_float("  42abc"), 42.0);
        assert_eq!(js_parse_float("-1.5e3x"), -1500.0);
        assert_eq!(js_parse_float(".5"), 0.5);
        assert_eq!(js_parse_float("5."), 5.0);
        assert_eq!(js_parse_float("1e"), 1.0);
        assert_eq!(js_parse_float("-Infinity"), f64::NEG_INFINITY);
        assert!(js_parse_float("").is_nan());
        assert!(js_parse_float("abc").is_nan());
        assert!(js_parse_float(".").is_nan());
    }
}
