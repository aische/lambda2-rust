use std::io::{Read, Write};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use lambda2::{RunConfig, Status, StdHost, Strategy, examples, load};

/// Run programs written in the lambda2 language (https://dvde.biz/lambda2/).
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Program file ('-' reads the program from stdin)
    #[arg(required_unless_present_any = ["eval", "example", "list_examples"])]
    file: Option<String>,

    /// Run PROGRAM given on the command line
    #[arg(short, long, value_name = "PROGRAM", conflicts_with_all = ["file", "example"])]
    eval: Option<String>,

    /// Run one of the website's examples, by number or name (see --list-examples)
    #[arg(long, value_name = "N|NAME", conflicts_with = "file")]
    example: Option<String>,

    /// List the built-in examples
    #[arg(long)]
    list_examples: bool,

    /// Evaluation strategy: lazy (need), cbn (name) or cbv (value)
    #[arg(short, long, default_value = "lazy", value_name = "STRATEGY")]
    strategy: Strategy,

    /// Stop after N reductions, 0 = unlimited
    #[arg(short, long, default_value_t = 1_000_000, value_name = "N")]
    max_steps: u64,

    /// Comma-separated numbers consumed by `get` before reading stdin
    #[arg(long, value_name = "NUMS", value_delimiter = ',')]
    input: Vec<f64>,

    /// Seed for `random`, for reproducible runs
    #[arg(long, value_name = "U64")]
    seed: Option<u64>,

    /// Print every step to stderr
    #[arg(long)]
    trace: bool,

    /// Maximum number of nodes printed per term
    #[arg(long, default_value_t = 2000, value_name = "N")]
    print_limit: usize,

    /// Do not print the statistics line
    #[arg(short, long)]
    quiet: bool,
}

const EXIT_USAGE: u8 = 1;
const EXIT_RUNTIME: u8 = 2;
const EXIT_LIMIT: u8 = 3;

/// The machine's substitution functions recurse over terms that can get deep (especially under
/// call-by-name), so the interpreter runs on a thread with a big stack.
const STACK_SIZE: usize = 256 * 1024 * 1024;

fn main() -> ExitCode {
    let cli = Cli::parse();
    std::thread::Builder::new()
        .name("lambda2".to_string())
        .stack_size(STACK_SIZE)
        .spawn(move || run(cli))
        .expect("failed to spawn the interpreter thread")
        .join()
        .unwrap_or(ExitCode::from(EXIT_RUNTIME))
}

/// Prints the result line; a closed stdout (e.g. `| head`) is not an error.
fn print_result(line: &str) {
    let _ = writeln!(std::io::stdout(), "{line}");
}

fn read_program(cli: &Cli) -> Result<String, String> {
    if let Some(src) = &cli.eval {
        return Ok(src.clone());
    }
    if let Some(key) = &cli.example {
        return examples::find(key)
            .map(|e| e.source.to_string())
            .ok_or_else(|| format!("unknown example '{key}' (see --list-examples)"));
    }
    match cli.file.as_deref() {
        Some("-") => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).map_err(|e| format!("cannot read stdin: {e}"))?;
            Ok(s)
        }
        Some(path) => std::fs::read_to_string(path).map_err(|e| format!("cannot read '{path}': {e}")),
        None => unreachable!("clap requires a program source"),
    }
}

fn run(cli: Cli) -> ExitCode {
    if cli.list_examples {
        for e in examples::EXAMPLES {
            println!("{:>2}  {:<18} {}", e.number, e.slug, e.title);
        }
        return ExitCode::SUCCESS;
    }
    let src = match read_program(&cli) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let (mut machine, free) = match load(&src, cli.strategy) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("parse error: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    for name in free {
        eprintln!("warning: free variable '{name}'");
    }

    let seed = cli
        .seed
        .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0));
    let mut host = StdHost::new(cli.input.clone(), seed);
    let cfg = RunConfig { max_steps: cli.max_steps, trace: cli.trace, print_limit: cli.print_limit };
    let status = machine.run(&cfg, &mut host);

    let result = machine.print(machine.root(), cli.print_limit);
    let (label, code) = match &status {
        Status::Done => {
            print_result(&format!("=> {result}"));
            ("done".to_string(), ExitCode::SUCCESS)
        }
        Status::StepLimit => {
            print_result(&format!("=> (incomplete) {result}"));
            eprintln!("stopped: maximum of {} reductions reached (use --max-steps to raise it)", cli.max_steps);
            ("stopped".to_string(), ExitCode::from(EXIT_LIMIT))
        }
        Status::Error(e) => {
            eprintln!("error: {e}");
            ("error".to_string(), ExitCode::from(EXIT_RUNTIME))
        }
    };
    if !cli.quiet {
        let mut stats = format!("{} · {label} · {} reductions", cli.strategy, machine.steps);
        if machine.workers_spawned > 0 {
            stats += &format!(" · {} workers, {} worker steps", machine.workers_spawned, machine.worker_steps);
            let running = machine.running_workers();
            if running > 0 {
                stats += &format!(" ({running} still running)");
            }
        }
        eprintln!("{stats}");
    }
    code
}
