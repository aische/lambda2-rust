//! Interpreter for the lambda2 language of the animated evaluator at <https://dvde.biz/lambda2/>.
//!
//! The evaluator is a step-by-step port of the website's machine, so results, output and
//! reduction counts match the site for every program that terminates there.

pub mod arena;
pub mod examples;
pub mod host;
pub mod lexer;
pub mod machine;
pub mod parser;
pub mod prim;
pub mod printer;
pub mod rng;
pub mod strategy;
pub mod subst;

pub use host::{Host, StdHost, TestHost};
pub use machine::{Machine, RunConfig, RuntimeError, Status};
pub use parser::ParseError;
pub use strategy::Strategy;

/// Parses `src` for `strategy` and returns a machine ready to run it, plus the program's free
/// variables (not an error: evaluation only fails if it reaches one).
pub fn load(src: &str, strategy: Strategy) -> Result<(Machine, Vec<String>), ParseError> {
    let mut arena = arena::Arena::new();
    let root = parser::parse(src, strategy, &mut arena)?;
    let free = subst::free_vars(&arena, root).into_iter().map(|s| arena.syms.name(s).to_string()).collect();
    Ok((Machine::new(arena, root), free))
}
