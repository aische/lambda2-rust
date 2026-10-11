//! The interpreter's connection to the outside world: `put`/`alert`, `get` and `random`.

use std::collections::VecDeque;
use std::io::{self, BufRead, IsTerminal, Write};

use crate::prim::{format_js, js_parse_float};
use crate::rng::Mulberry32;

pub trait Host {
    /// `put n` / `alert n`.
    fn output(&mut self, n: f64);
    /// `get`. An error stops the program.
    fn input(&mut self) -> Result<f64, String>;
    /// `random`: uniform in [0, 1).
    fn random(&mut self) -> f64;
    /// A problem that does not stop the program (e.g. a failed `par` worker).
    fn warning(&mut self, msg: &str) {
        eprintln!("warning: {msg}");
    }
    /// One `--trace` line.
    fn trace(&mut self, line: &str) {
        eprintln!("{line}");
    }
}

/// Console host: output to stdout (flushed per value, so infinite programs stream), input from
/// `--input` values and then stdin.
pub struct StdHost {
    inputs: VecDeque<f64>,
    rng: Mulberry32,
}

impl StdHost {
    pub fn new(inputs: Vec<f64>, seed: u64) -> StdHost {
        StdHost { inputs: inputs.into(), rng: Mulberry32::new(seed) }
    }
}

impl Host for StdHost {
    fn output(&mut self, n: f64) {
        let mut out = io::stdout().lock();
        // A closed pipe (e.g. `| head`) is not an interpreter error; stop quietly.
        if writeln!(out, "{}", format_js(n)).and_then(|_| out.flush()).is_err() {
            std::process::exit(0);
        }
    }

    fn input(&mut self) -> Result<f64, String> {
        if let Some(n) = self.inputs.pop_front() {
            return Ok(n);
        }
        let stdin = io::stdin();
        if stdin.is_terminal() {
            eprint!("enter a number: ");
            let _ = io::stderr().flush();
        }
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => Err("get: no more input".to_string()),
            Ok(_) => Ok(js_parse_float(&line)),
            Err(e) => Err(format!("get: {e}")),
        }
    }

    fn random(&mut self) -> f64 {
        self.rng.next_f64()
    }
}

/// Host for tests: collects output, takes scripted input.
pub struct TestHost {
    pub output: Vec<f64>,
    pub inputs: VecDeque<f64>,
    pub warnings: Vec<String>,
    pub trace: Vec<String>,
    rng: Mulberry32,
}

impl TestHost {
    pub fn new(inputs: Vec<f64>, seed: u64) -> TestHost {
        TestHost {
            output: Vec::new(),
            inputs: inputs.into(),
            warnings: Vec::new(),
            trace: Vec::new(),
            rng: Mulberry32::new(seed),
        }
    }
}

impl Host for TestHost {
    fn output(&mut self, n: f64) {
        self.output.push(n);
    }

    fn input(&mut self) -> Result<f64, String> {
        // Like the oracle harness, whose `prompt` returns "" (→ NaN) when the script runs out.
        Ok(self.inputs.pop_front().unwrap_or(f64::NAN))
    }

    fn random(&mut self) -> f64 {
        self.rng.next_f64()
    }

    fn warning(&mut self, msg: &str) {
        self.warnings.push(msg.to_string());
    }

    fn trace(&mut self, line: &str) {
        self.trace.push(line.to_string());
    }
}
