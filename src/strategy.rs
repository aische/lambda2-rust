use std::fmt;
use std::str::FromStr;

/// Evaluation strategy. Chosen at parse time, because the parser desugars differently per strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    /// Call-by-need: shared, memoised thunks; IO actions are values run by top-level `bind`s.
    Lazy,
    /// Call-by-name: arguments are copied unevaluated into the body.
    ByName,
    /// Call-by-value: arguments (and both halves of a cons) are evaluated before substitution.
    ByValue,
}

impl Strategy {
    pub fn name(self) -> &'static str {
        match self {
            Strategy::Lazy => "lazy",
            Strategy::ByName => "cbn",
            Strategy::ByValue => "cbv",
        }
    }
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Strategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lazy" | "need" => Ok(Strategy::Lazy),
            "cbn" | "name" => Ok(Strategy::ByName),
            "cbv" | "value" => Ok(Strategy::ByValue),
            _ => Err(format!("unknown strategy '{s}' (expected lazy, cbn or cbv)")),
        }
    }
}
