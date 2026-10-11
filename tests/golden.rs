//! Compares every example under every strategy with golden files produced by the website's own
//! evaluator (`node tools/oracle/run.js`): status, reduction count, output and printed result.

use lambda2::examples::EXAMPLES;
use lambda2::prim::format_js;
use lambda2::printer::{PrintOptions, print_term};
use lambda2::{RunConfig, Status, Strategy, TestHost, load};

// Must match the defaults of tools/oracle/run.js.
const MAX_STEPS: u64 = 100_000;
const SEED: u64 = 42;
const INPUT: &[f64] = &[5.0];
const PRINT_LIMIT: usize = 300;

struct Golden {
    status: String,
    steps: u64,
    result: String,
    output: Vec<String>,
}

fn read_golden(number: u32, strategy: Strategy) -> Golden {
    let path = format!("{}/tests/golden/{number:02}-{strategy}.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut lines = text.lines();
    let mut field = |name: &str| {
        let line = lines.next().unwrap();
        line.strip_prefix(name)
            .and_then(|l| l.strip_prefix(' '))
            .unwrap_or_else(|| panic!("{path}: {line}"))
            .to_string()
    };
    let status = field("status");
    let steps = field("steps").parse().unwrap();
    let result = field("result");
    assert_eq!(lines.next(), Some("output"));
    Golden { status, steps, result, output: lines.map(str::to_string).collect() }
}

/// Runs one example and compares it with its golden file. Returns the number of garbage collections.
fn check(number: u32, strategy: Strategy, gc_min_allocs: Option<usize>) -> Result<u64, String> {
    let ex = &EXAMPLES[number as usize];
    let golden = read_golden(number, strategy);
    let (mut machine, _) = match load(ex.source, strategy) {
        Ok(m) => m,
        Err(e) if golden.status == "parse-error" => {
            assert!(e.message.starts_with("par is only allowed"), "{e}");
            return Ok(0);
        }
        Err(e) => return Err(format!("parse error: {e}")),
    };
    if let Some(n) = gc_min_allocs {
        machine.arena.gc_min_allocs = n;
    }
    let mut host = TestHost::new(INPUT.to_vec(), SEED);
    let cfg = RunConfig { max_steps: MAX_STEPS, ..RunConfig::default() };
    let status = machine.run(&cfg, &mut host);
    let status = match status {
        Status::Done => "done",
        Status::StepLimit => "limit",
        Status::Error(e) => return Err(format!("runtime error: {e}")),
    };
    let output: Vec<String> = host.output.iter().map(|&n| format_js(n)).collect();
    let result = print_term(&machine.arena, machine.root(), &PrintOptions { limit: PRINT_LIMIT, ..Default::default() });

    let mut problems = Vec::new();
    if status != golden.status {
        problems.push(format!("status {status}, expected {}", golden.status));
    }
    if machine.steps != golden.steps {
        problems.push(format!("{} steps, expected {}", machine.steps, golden.steps));
    }
    if output != golden.output {
        problems.push(format!("output {output:?}, expected {:?}", golden.output));
    }
    if result != golden.result {
        problems.push(format!("result\n    {result}\n  expected\n    {}", golden.result));
    }
    if problems.is_empty() { Ok(machine.arena.collections) } else { Err(problems.join("; ")) }
}

/// Checks every example under every strategy and returns the total number of collections.
fn check_all(gc_min_allocs: Option<usize>) -> u64 {
    let mut failures = Vec::new();
    let mut collections = 0;
    for ex in EXAMPLES {
        for strategy in [Strategy::Lazy, Strategy::ByName, Strategy::ByValue] {
            match check(ex.number, strategy, gc_min_allocs) {
                Ok(n) => collections += n,
                Err(e) => failures.push(format!("{:02}-{strategy} ({}): {e}", ex.number, ex.slug)),
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
    collections
}

#[test]
fn examples_match_the_website() {
    check_all(None);
}

/// With the default threshold only a few of the heaviest runs above collect at all. Collecting after
/// every few allocations, in every run, checks that the collector keeps everything the machine still
/// uses (a freed node would crash the machine or change the result).
#[test]
fn examples_match_the_website_with_frequent_gc() {
    assert!(check_all(Some(64)) > 1000);
}
