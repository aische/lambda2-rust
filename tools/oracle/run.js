// Runs the website's own evaluator (site/*.js, copied unchanged from https://dvde.biz/lambda2/)
// headlessly and writes golden files for the Rust test suite.
//
// usage: node tools/oracle/run.js [--max-steps N] [--seed S] [--input 5,3] [--out tests/golden]
//
// For every example x strategy it writes <out>/<NN>-<strategy>.txt:
//
//   status done|limit|freevar|stopped|crash|parse-error
//   steps <reductions>
//   result <printed term>
//   output
//   <one put/alert value per line>

"use strict";
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const args = process.argv.slice(2);
function opt(name, def) {
    const i = args.indexOf(name);
    return i >= 0 ? args[i + 1] : def;
}
const MAX_STEPS = Number(opt("--max-steps", "100000"));
const SEED = Number(opt("--seed", "42"));
const INPUT = opt("--input", "5").split(",").filter((s) => s !== "").map(Number);
const PRINT_LIMIT = Number(opt("--print-limit", "300"));
const ROOT = path.join(__dirname, "..", "..");
const OUT = path.resolve(opt("--out", path.join(ROOT, "tests", "golden")));

// Same algorithm and seeding as src/rng.rs.
function mulberry32(seed) {
    let a = seed >>> 0;
    return function () {
        a = (a + 0x6d2b79f5) | 0;
        let t = Math.imul(a ^ (a >>> 15), 1 | a);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

function makeContext() {
    const ctx = { console };
    vm.createContext(ctx);
    for (const f of ["LamNode.js", "LamTypes.js", "LamParser.js", "LamEvaluator.js"]) {
        vm.runInContext(fs.readFileSync(path.join(__dirname, "site", f), "utf8"), ctx, { filename: f });
    }
    return ctx;
}

// Must print exactly like src/printer.rs (non-trace mode).
function printTerm(ctx, term, limit) {
    const T = (name) => ctx[name];
    const out = [];
    const onPath = new Set();
    let count = 0;
    const ABORT = {};
    function go(n) {
        if (n === undefined) {
            out.push("?");
            return;
        }
        if (n.type === T("LamThunk")) {
            if (onPath.has(n)) {
                out.push("…");
                return;
            }
            onPath.add(n);
            go(n.nodes[0]);
            onPath.delete(n);
            return;
        }
        count++;
        if (count > limit) {
            out.push("…");
            throw ABORT;
        }
        switch (n.type) {
            case T("LamVar"): out.push(n.name); return;
            case T("LamAbs"): out.push("\\" + n.name + "."); go(n.nodes[0]); return;
            case T("LamRec"): out.push("Y " + n.name + "."); go(n.nodes[0]); return;
            case T("LamApp"):
            case T("LamAps"): out.push("@"); go(n.nodes[0]); go(n.nodes[1]); return;
            case T("LamNum"): out.push(String(n.number)); return;
            case T("LamOp0"): out.push(n.opname); return;
            case T("LamOp1"): out.push(n.opname); go(n.nodes[0]); return;
            case T("LamOp2"): out.push(n.opname); go(n.nodes[0]); go(n.nodes[1]); return;
            case T("LamIf"):
                out.push("if"); go(n.nodes[0]); out.push("then"); go(n.nodes[1]); out.push("else"); go(n.nodes[2]);
                return;
            case T("LamCons"): out.push(":"); go(n.nodes[0]); go(n.nodes[1]); return;
            case T("LamCar"): out.push("car"); go(n.nodes[0]); return;
            case T("LamCdr"): out.push("cdr"); go(n.nodes[0]); return;
            case T("LamIscons"): out.push("iscons"); go(n.nodes[0]); return;
            case T("LamUnit"): out.push("return"); go(n.nodes[0]); return;
            case T("LamBind"): out.push("bind"); go(n.nodes[0]); go(n.nodes[1]); return;
            case T("LamPar"): out.push("par"); go(n.nodes[0]); go(n.nodes[1]); return;
        }
        out.push("?");
    }
    try {
        go(term);
    } catch (e) {
        if (e !== ABORT) throw e;
    }
    return out.join(" ");
}

function run(source, strategy) {
    const ctx = makeContext();
    const output = [];
    const input = INPUT.slice();
    ctx.prompt = () => (input.length > 0 ? String(input.shift()) : "");
    ctx.alert = (n) => output.push(String(n));
    ctx.__random = mulberry32(SEED);
    vm.runInContext("Math.random = __random", ctx);
    vm.runInContext("LamNode.idCount = 0", ctx);

    const parser = new ctx.LamParser();
    const term = parser.parse(source, strategy === "lazy", strategy === "cbv");
    if (!term) {
        return { status: "parse-error", steps: 0, result: parser.error || "cannot parse", output };
    }
    const ev = new ctx.Evaluator(term, (n) => output.push(String(n)));
    let steps = 0;
    let status;
    try {
        let cur = ev.step();
        for (;;) {
            if (steps === MAX_STEPS) {
                status = "limit";
                break;
            }
            const kind = cur[0].name;
            const ok = cur[0].action();
            for (let i = 1; i < cur.length; i++) {
                cur[i].action();
            }
            if (!ok) {
                status = ev.done ? "done" : kind === "freevar" ? "freevar" : "stopped";
                break;
            }
            // `cons1` at the root pops (done) but reports success; the site would loop forever here.
            if (ev.done) {
                status = "done";
                break;
            }
            steps++;
            cur = ev.step();
        }
    } catch (e) {
        status = "crash";
        return { status, steps, result: String(e && e.message), output };
    }
    return { status, steps, result: printTerm(ctx, ev.root(), PRINT_LIMIT), output };
}

const exDir = path.join(ROOT, "examples");
const files = fs.readdirSync(exDir).filter((f) => f.endsWith(".lam")).sort();
fs.mkdirSync(OUT, { recursive: true });
for (const f of files) {
    const source = fs.readFileSync(path.join(exDir, f), "utf8");
    for (const strategy of ["lazy", "cbn", "cbv"]) {
        const r = run(source, strategy);
        const name = f.slice(0, 2) + "-" + strategy + ".txt";
        const text =
            `status ${r.status}\nsteps ${r.steps}\nresult ${r.result}\noutput\n` +
            r.output.map((s) => s + "\n").join("");
        fs.writeFileSync(path.join(OUT, name), text);
        console.log(`${name}: ${r.status} ${r.steps} ${r.output.length} outputs`);
    }
}
