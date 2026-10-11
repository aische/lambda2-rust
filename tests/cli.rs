use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

fn lambda2() -> Command {
    Command::cargo_bin("lambda2").unwrap()
}

#[test]
fn runs_an_example_with_stats() {
    lambda2().args(["--example", "0"]).assert().success().stdout("=> 100\n").stderr("lazy · done · 11 reductions\n");
}

#[test]
fn strategies_and_aliases() {
    for (s, steps) in [("cbn", 8), ("name", 8), ("cbv", 10), ("value", 10), ("need", 11)] {
        lambda2()
            .args(["--example", "simple-function-1", "-s", s])
            .assert()
            .success()
            .stderr(contains(format!("· {steps} reductions")));
    }
}

#[test]
fn put_streams_output_before_the_result() {
    lambda2()
        .args(["-q", "-e", "bind put 1 \\x. bind put 2 \\y. return 0"])
        .assert()
        .success()
        .stdout("1\n2\n=> return 0\n")
        .stderr("");
}

#[test]
fn step_limit() {
    lambda2()
        .args(["--example", "prime-numbers", "-m", "2000"])
        .assert()
        .code(3)
        .stdout(predicate::str::starts_with("2\n3\n5\n").and(contains("=> (incomplete) ")))
        .stderr(
            contains("stopped: maximum of 2000 reductions reached").and(contains("lazy · stopped · 2000 reductions")),
        );
}

#[test]
fn get_reads_input_option_then_stdin() {
    lambda2()
        .args(["-q", "-s", "cbv", "-e", "+ get get", "--input", "1"])
        .write_stdin("41\n")
        .assert()
        .success()
        .stdout("=> 42\n");
    lambda2()
        .args(["-q", "-s", "cbv", "-e", "get"])
        .write_stdin("")
        .assert()
        .code(2)
        .stderr("error: get: no more input\n");
}

#[test]
fn program_from_stdin() {
    lambda2().args(["-q", "-"]).write_stdin("* 6 7").assert().success().stdout("=> 42\n");
}

#[test]
fn seeded_random_is_reproducible() {
    let run = || lambda2().args(["-q", "-s", "cbn", "-e", "random", "--seed", "42"]).output().unwrap().stdout;
    assert_eq!(run(), b"=> 0.6011037519201636\n");
    assert_eq!(run(), run());
}

#[test]
fn parse_errors_exit_1() {
    lambda2()
        .args(["-e", "if 1 else 2"])
        .assert()
        .code(1)
        .stderr("parse error: line 1:6: expected 'then', found 'else'\n");
    lambda2()
        .args(["--example", "10", "-s", "cbv"])
        .assert()
        .code(1)
        .stderr(contains("par is only allowed for lazy evaluation"));
    lambda2().arg("no-such-file.lam").assert().code(1).stderr(contains("cannot read 'no-such-file.lam'"));
    lambda2().args(["--example", "99"]).assert().code(1).stderr(contains("unknown example '99'"));
}

#[test]
fn runtime_errors_exit_2() {
    let cases = [
        ("@ \\x. y 1", "error: free variable 'y'"),
        ("@ 1 2", "error: cannot apply non-function: 1"),
        ("car 5", "error: car of non-pair: 5"),
        ("+ \\x.x 1", "error: '+' expects numbers, got \\x. x"),
        ("bind 1 \\x.x", "error: bind expects an IO action, got 1"),
        ("letrec x = + x 1 in x", "error: infinite loop"),
    ];
    for (program, message) in cases {
        lambda2().args(["-q", "-e", program]).assert().code(2).stderr(contains(message));
    }
}

#[test]
fn free_variables_are_reported_before_running() {
    lambda2().args(["-q", "-e", "if 1 then 2 else y"]).assert().success().stderr("warning: free variable 'y'\n");
}

#[test]
fn par_stats_and_failed_workers() {
    lambda2()
        .args(["--example", "parallel-1"])
        .assert()
        .success()
        .stdout("=> 102340920\n")
        .stderr(contains("lazy · done · 3170 reductions · 2 workers, 3868 worker steps (1 still running)"));
    lambda2()
        .args(["-q", "-e", "par car 1 @ \\x. @ \\y. y 7 1"])
        .assert()
        .success()
        .stdout("=> 7\n")
        .stderr("warning: parallel evaluation failed: car of non-pair: 1\n");
}

#[test]
fn trace_prints_every_step() {
    let out = lambda2().args(["--example", "0", "--trace", "-q"]).output().unwrap();
    let trace = String::from_utf8(out.stderr).unwrap();
    let kinds: Vec<&str> = trace.lines().map(|l| l.split_whitespace().nth(3).unwrap()).collect();
    assert_eq!(kinds, ["app0", "abs", "app1", "op2_0", "thunk0", "num", "thunk1", "num", "op2_0", "num", "op2", "num"]);
    assert!(trace.starts_with("#     1 main app0     ⟨@ \\x. + x 1 99⟩\n"));
}

#[test]
fn list_examples() {
    lambda2()
        .arg("--list-examples")
        .assert()
        .success()
        .stdout(contains(" 0  simple-function-1  simple function I\n").and(contains("17  merge-sort")));
}
