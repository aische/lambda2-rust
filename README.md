# lambda2

A command-line interpreter for the small lambda-calculus language of the animated evaluator at <https://github.com/aische/lambda2> (<https://dvde.biz/lambda2/>).

It ports the site's step-by-step machine one step at a time. For every program that terminates on the website,
the result, the output and the number of reductions are the same.

## Usage

```
cargo run --release -- [OPTIONS] <FILE>          # '-' reads the program from stdin
cargo run --release -- [OPTIONS] -e '<PROGRAM>'
cargo run --release -- [OPTIONS] --example <N|NAME>
cargo run --release -- --list-examples

  -s, --strategy <lazy|cbn|cbv>  evaluation strategy [default: lazy] (aliases: need, name, value)
  -m, --max-steps <N>            stop after N reductions, 0 = unlimited [default: 1000000]
      --input <NUMS>             comma-separated values consumed by `get` before stdin
      --seed <U64>               seed for `random`
      --trace                    print every step to stderr
      --print-limit <N>          max nodes when printing terms [default: 2000]
  -q, --quiet                    no statistics line
```

- stdout gets one line per `put`/`alert` value, as it happens, and then the result as `=> <term>`.
- stderr gets warnings, the trace and a statistics line such as `lazy · done · 11 reductions`.
- Exit codes: 0 finished, 1 usage/parse error, 2 runtime error, 3 step limit reached.
- If stdout is closed while `put`/`alert` write to it (e.g. `| head`), the run stops quietly with exit code 0 and
  no statistics line.

```
$ lambda2 --example fibonacci-1
=> 55
lazy · done · 742 reductions
```

## Language

```
x                      variable (lowercase letters)
@ f a                  application
\x. T                  abstraction
Y x. T                 recursion
123                    number
+ - * / % ^ < <= > >= == <>   T T
not log sqrt floor ceil round T
if T then T else T     (0 is false)
: H T   car T   cdr T   iscons T      lists end with 0
let x = A in B         letrec x = A in B
bind A B   return T   put T   alert T   get   random
par A B                evaluate A in parallel, continue with B (lazy only)
# comment
```

All operators are prefix with a fixed number of arguments, so parentheses are optional.

Under `lazy`, IO actions are values and only run when a top-level `bind` chain runs them, exactly as on the site.
So `put 5` alone evaluates to `return put 5` and prints nothing. Under `cbn`/`cbv`, `put`, `get` and `random` run
whenever they are evaluated.

`cbn` and `cbv` substitute by copying the argument into every place it is used, like the site. That keeps the
reduction counts identical, but it makes them slow on large data: a function that walks a list copies the rest of
the list at every step, so the run time grows with the square of the list length. Use `lazy` for big inputs.

## Differences from the website

These only matter for programs that crash, hang or silently compute garbage on the site:

- Type errors (applying a number, `car` of a non-pair, arithmetic on a function, …) are reported as errors instead
  of crashing or producing `NaN`.
- Substitution under `cbn`/`cbv` also handles `Y`, so it does not crash.
- A failing `par` worker is dropped with a warning.
- Blackholes (`letrec x = + x 1 in x`) and deadlocks are detected instead of blocking forever.
- A cons at the root ends the run (on the site the run loop would never stop).
- Parse errors have positions, and keywords cannot be used as variable names.
- Negative results print as `-5`, which is readable but cannot be parsed back (there are no negative literals).

## Development

```
cargo test                      # unit, CLI and golden tests
node tools/oracle/run.js        # regenerate tests/golden from the site's own JS (tools/oracle/site)
```

The golden suite runs every example under every strategy and compares the status, reduction count, output and
printed result with what the original JavaScript evaluator produces.
