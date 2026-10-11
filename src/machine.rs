//! The evaluator: a port of the site's focus + stack machine (`LamEvaluator.js`) and of its run
//! loop (`LamMain.js`), one step at a time, so that reduction counts match the website.
//!
//! Each tick first *decides* a step for every evaluator (main first, then the `par` workers) and
//! then *applies* them in the same order. Deciding is not side-effect free: `thunk0` marks the
//! thunk as being evaluated right away, so a second evaluator reaching it in the same tick blocks.

use crate::arena::{Arena, NodeId, Tag};
use crate::host::Host;
use crate::prim::{self, Op};
use crate::printer::{PrintOptions, print_term};
use crate::subst;

/// Step kinds, named as on the website.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepKind {
    FreeVar,
    Abs,
    App0,
    App1,
    Rec,
    Num,
    Op0,
    Op1_0,
    Op1,
    Op2_0,
    Op2,
    If0,
    If1,
    Cons1,
    Cons0,
    Car0,
    Car1,
    Cdr0,
    Cdr1,
    IsCons0,
    IsCons1,
    Unit,
    Bind,
    Bind0,
    Bind1,
    Thunk0,
    Thunk1,
    Block,
    Par,
}

impl StepKind {
    pub fn name(self) -> &'static str {
        use StepKind::*;
        match self {
            FreeVar => "freevar",
            Abs => "abs",
            App0 => "app0",
            App1 => "app1",
            Rec => "rec",
            Num => "num",
            Op0 => "op0",
            Op1_0 => "op1_0",
            Op1 => "op1",
            Op2_0 => "op2_0",
            Op2 => "op2",
            If0 => "if0",
            If1 => "if1",
            Cons1 => "cons1",
            Cons0 => "cons0",
            Car0 => "car0",
            Car1 => "car1",
            Cdr0 => "cdr0",
            Cdr1 => "cdr1",
            IsCons0 => "iscons0",
            IsCons1 => "iscons1",
            Unit => "unit",
            Bind => "bind",
            Bind0 => "bind0",
            Bind1 => "bind1",
            Thunk0 => "thunk0",
            Thunk1 => "thunk1",
            Block => "block",
            Par => "par",
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RuntimeError {
    #[error("free variable '{0}'")]
    FreeVar(String),
    #[error("cannot apply non-function: {0}")]
    NotAFunction(String),
    #[error("{op} of non-pair: {term}")]
    NotAPair { op: &'static str, term: String },
    #[error("'{op}' expects numbers, got {term}")]
    NotANumber { op: Op, term: String },
    #[error("bind expects an IO action, got {0}")]
    BindNotIo(String),
    #[error("infinite loop: a value depends on itself (blackhole)")]
    Blackhole,
    #[error("deadlock: every evaluator is waiting for a value another one is computing")]
    Deadlock,
    #[error("waiting for a value whose parallel evaluation failed")]
    Orphaned,
    #[error("{0}")]
    Input(String),
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Done,
    StepLimit,
    Error(RuntimeError),
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Stop after this many reductions; 0 means no limit.
    pub max_steps: u64,
    /// Report every step through [`Host::trace`].
    pub trace: bool,
    /// Node limit for printed terms (trace lines, error messages use a smaller one).
    pub print_limit: usize,
}

impl Default for RunConfig {
    fn default() -> Self {
        RunConfig { max_steps: 1_000_000, trace: false, print_limit: 2000 }
    }
}

struct Evaluator {
    id: u32,
    /// The focus.
    term: NodeId,
    /// Ancestors of the focus; `stack[0]` is the root.
    stack: Vec<NodeId>,
    done: bool,
}

impl Evaluator {
    fn new(id: u32, term: NodeId) -> Evaluator {
        Evaluator { id, term, stack: Vec::new(), done: false }
    }

    fn root(&self) -> NodeId {
        self.stack.first().copied().unwrap_or(self.term)
    }
}

/// Error messages show at most this many nodes of the offending term.
const ERROR_PRINT_LIMIT: usize = 30;

pub struct Machine {
    pub arena: Arena,
    /// `evals[0]` is the main evaluator, the rest are `par` workers in spawn order.
    evals: Vec<Evaluator>,
    next_eval_id: u32,
    /// Counter for alpha-renamed binders (`x_0`, `x_1`, …); the JS `LamNode.nextID`.
    alpha: u32,
    /// Ticks whose main step succeeded: the website's "reductions".
    pub steps: u64,
    /// Steps taken by `par` workers.
    pub worker_steps: u64,
    pub workers_spawned: u32,
}

impl Machine {
    pub fn new(arena: Arena, root: NodeId) -> Machine {
        Machine {
            arena,
            evals: vec![Evaluator::new(0, root)],
            next_eval_id: 1,
            alpha: 0,
            steps: 0,
            worker_steps: 0,
            workers_spawned: 0,
        }
    }

    /// The main evaluator's whole term (the result once the run is done).
    pub fn root(&self) -> NodeId {
        self.evals[0].root()
    }

    /// Workers that have not finished.
    pub fn running_workers(&self) -> usize {
        self.evals[1..].iter().filter(|e| !e.done).count()
    }

    pub fn print(&self, node: NodeId, limit: usize) -> String {
        print_term(&self.arena, node, &PrintOptions { limit, ..PrintOptions::default() })
    }

    pub fn run(&mut self, cfg: &RunConfig, host: &mut dyn Host) -> Status {
        loop {
            if cfg.max_steps != 0 && self.steps >= cfg.max_steps {
                return Status::StepLimit;
            }
            if self.arena.wants_gc() {
                self.gc();
            }
            match self.tick(cfg, host) {
                Ok(true) => self.steps += 1,
                Ok(false) => return Status::Done,
                Err(e) => return Status::Error(e),
            }
        }
    }

    fn gc(&mut self) {
        let roots: Vec<NodeId> =
            self.evals.iter().flat_map(|e| e.stack.iter().copied().chain(std::iter::once(e.term))).collect();
        self.arena.collect(roots);
    }

    /// One tick. `Ok(false)` when the main evaluator has finished.
    fn tick(&mut self, cfg: &RunConfig, host: &mut dyn Host) -> Result<bool, RuntimeError> {
        let mut i = 1;
        self.evals.retain(|e| {
            let keep = i == 1 || !e.done;
            i += 1;
            keep
        });

        // Decide. Workers spawned while applying start next tick, so remember how many there are.
        let n = self.evals.len();
        let mut kinds: Vec<Result<StepKind, RuntimeError>> = Vec::with_capacity(n);
        for ix in 0..n {
            let kind = self.decide(ix);
            if ix > 0 {
                self.worker_steps += 1;
            }
            if cfg.trace {
                self.trace_line(ix, &kind, cfg, host);
            }
            kinds.push(kind);
        }
        if kinds.iter().all(|k| matches!(k, Ok(StepKind::Block))) {
            return Err(RuntimeError::Deadlock);
        }

        // Apply. Like the site, the workers still take their step when the main evaluator stops.
        let mut main_result = Ok(true);
        for (ix, kind) in kinds.into_iter().enumerate() {
            let result = kind.and_then(|k| self.apply(ix, k, host));
            if ix == 0 {
                main_result = result;
            } else if let Err(e) = result {
                host.warning(&format!("parallel evaluation failed: {e}"));
                self.evals[ix].done = true;
            }
        }
        main_result
    }

    fn trace_line(&self, ix: usize, kind: &Result<StepKind, RuntimeError>, cfg: &RunConfig, host: &mut dyn Host) {
        let ev = &self.evals[ix];
        let who = if ix == 0 { "main".to_string() } else { format!("w{}", ev.id) };
        let name = match kind {
            Ok(k) => k.name(),
            Err(_) => "block",
        };
        let mut path = ev.stack.clone();
        path.push(ev.term);
        let term = print_term(
            &self.arena,
            ev.root(),
            &PrintOptions { limit: cfg.print_limit, trace: true, focus: Some(&path) },
        );
        host.trace(&format!("#{:>6} {who:<4} {name:<8} {term}", self.steps + 1));
    }

    /// The JS `stepE`: which step the evaluator takes next.
    fn decide(&mut self, ix: usize) -> Result<StepKind, RuntimeError> {
        use StepKind as K;
        let ev = &self.evals[ix];
        let node = self.arena.get(ev.term);
        let es = node.eval_state;
        Ok(match node.tag {
            Tag::Var => K::FreeVar,
            Tag::Abs => K::Abs,
            Tag::Num => K::Num,
            Tag::Unit => K::Unit,
            Tag::App => {
                if es < 1 {
                    K::App0
                } else {
                    K::App1
                }
            }
            Tag::Aps => {
                if es < 2 {
                    K::App0
                } else {
                    K::App1
                }
            }
            Tag::Rec => K::Rec,
            Tag::Op0 => K::Op0,
            Tag::Op1 => {
                if es < 1 {
                    K::Op1_0
                } else {
                    K::Op1
                }
            }
            Tag::Op2 => {
                if es < 2 {
                    K::Op2_0
                } else {
                    K::Op2
                }
            }
            Tag::If => {
                if es < 1 {
                    K::If0
                } else {
                    K::If1
                }
            }
            Tag::Cons => {
                if es == 0 {
                    K::Cons1
                } else {
                    K::Cons0
                }
            }
            Tag::Car => {
                if es < 1 {
                    K::Car0
                } else {
                    K::Car1
                }
            }
            Tag::Cdr => {
                if es < 1 {
                    K::Cdr0
                } else {
                    K::Cdr1
                }
            }
            Tag::IsCons => {
                if es < 1 {
                    K::IsCons0
                } else {
                    K::IsCons1
                }
            }
            Tag::Bind => {
                // An IO action at the top level, or the first part of a running bind, is run;
                // anywhere else it is a value.
                let io_continuation = ev.stack.last().is_none_or(|&p| self.arena.tag(p) == Tag::Bind);
                match (io_continuation, es) {
                    (false, _) => K::Bind,
                    (true, 0) => K::Bind0,
                    (true, _) => K::Bind1,
                }
            }
            Tag::Thunk if node.eval_flag => K::Thunk1,
            Tag::Thunk => match node.open_by {
                None => {
                    let id = ev.id;
                    self.arena.get_mut(ev.term).open_by = Some(id);
                    K::Thunk0
                }
                // On the site these spin forever in `block` steps.
                Some(owner) if owner == ev.id => return Err(RuntimeError::Blackhole),
                Some(owner) if !self.evals.iter().any(|e| e.id == owner && !e.done) => {
                    return Err(RuntimeError::Orphaned);
                }
                Some(_) => K::Block,
            },
            Tag::Par => K::Par,
            Tag::Free => unreachable!("focus on a freed node"),
        })
    }

    /// The step's action. `Ok(false)`: the evaluator finished (it popped at the root).
    fn apply(&mut self, ix: usize, kind: StepKind, host: &mut dyn Host) -> Result<bool, RuntimeError> {
        use StepKind as K;
        let a = &mut self.arena;
        let ev = &mut self.evals[ix];
        let term = ev.term;
        match kind {
            K::FreeVar => return Err(RuntimeError::FreeVar(a.syms.name(a.get(term).name).to_string())),
            K::Abs | K::Num | K::Unit | K::Bind | K::Cons0 => return Ok(pop(a, ev)),
            K::App0 | K::Op1_0 | K::Op2_0 | K::If0 | K::Car0 | K::Cdr0 | K::IsCons0 | K::Thunk0 => push(a, ev),
            K::Block => {}
            K::App1 => {
                let f = a.kid(term, 0);
                let arg = a.kid(term, 1);
                if a.tag(f) != Tag::Abs {
                    return Err(RuntimeError::NotAFunction(self.print(f, ERROR_PRINT_LIMIT)));
                }
                let (lazy, name, body) = {
                    let fnode = a.get(f);
                    (fnode.lazy, fnode.name, fnode.kids[0])
                };
                ev.term = if lazy {
                    // Share the argument through a closed thunk (call-by-need).
                    let t = match a.get(arg) {
                        n if n.tag == Tag::Bind || (n.tag == Tag::Thunk && n.closed) => arg,
                        _ => a.thunk(true, arg),
                    };
                    subst::substitute(a, body, name, t)
                } else {
                    let free = subst::free_vars(a, arg);
                    subst::substitute_clone_alpha(a, body, name, arg, &free, &mut self.alpha)
                };
                rebind(a, ev);
            }
            K::Rec => {
                // `Y x.T` becomes a closed thunk whose body refers to the thunk itself: a cycle.
                let (name, body) = (a.get(term).name, a.kid(term, 0));
                let t = a.thunk(true, crate::arena::NIL);
                let b = subst::substitute(a, body, name, t);
                a.get_mut(t).kids[0] = b;
                ev.term = t;
                rebind(a, ev);
            }
            K::Op0 => {
                let n = match a.get(term).op {
                    Op::Get => host.input().map_err(RuntimeError::Input)?,
                    _ => host.random(),
                };
                ev.term = a.num(n);
                rebind(a, ev);
            }
            K::Op1 => {
                let op = a.get(term).op;
                let x = self.number_arg(ix, op, 0)?;
                let r = match op {
                    Op::Put | Op::Alert => {
                        host.output(x);
                        0.0
                    }
                    _ => prim::op1(op, x),
                };
                self.replace_focus_with_num(ix, r);
            }
            K::Op2 => {
                let op = a.get(term).op;
                let x = self.number_arg(ix, op, 0)?;
                let y = self.number_arg(ix, op, 1)?;
                self.replace_focus_with_num(ix, prim::op2(op, x, y));
            }
            K::If1 => {
                let c = a.get(a.kid(term, 0));
                // Truthiness as on the site: only the number 0 is false (NaN and non-numbers are true).
                let k = if c.tag == Tag::Num && c.num == 0.0 { 2 } else { 1 };
                ev.term = a.kid(term, k);
                rebind(a, ev);
            }
            K::Cons1 => {
                // Quirk kept for exact step counts: the site checks the *root* (`stack[0]`), not
                // the parent. If the root is a `car`/`cdr`, the head/tail is evaluated now.
                match ev.stack.first().map(|&r| a.tag(r)) {
                    Some(Tag::Car) => push(a, ev),
                    Some(Tag::Cdr) => {
                        a.get_mut(term).eval_state = 1;
                        push(a, ev);
                    }
                    // The site ignores the result here and would loop forever on a cons at the root.
                    _ => return Ok(pop(a, ev)),
                }
            }
            K::Car1 | K::Cdr1 => {
                let c = a.kid(term, 0);
                if a.tag(c) != Tag::Cons {
                    let op = if kind == K::Car1 { "car" } else { "cdr" };
                    return Err(RuntimeError::NotAPair { op, term: self.print(c, ERROR_PRINT_LIMIT) });
                }
                ev.term = a.kid(c, if kind == K::Car1 { 0 } else { 1 });
                rebind(a, ev);
            }
            K::IsCons1 => {
                let is = a.tag(a.kid(term, 0)) == Tag::Cons;
                self.replace_focus_with_num(ix, if is { 1.0 } else { 0.0 });
            }
            K::Bind0 => {
                // A bind may be shared, so it is copied before it is run.
                let node = a.get(term);
                let (x, f, flag) = (node.kids[0], node.kids[1], node.eval_flag);
                ev.term = a.bind(x, f, flag);
                rebind(a, ev);
                push(a, ev);
            }
            K::Bind1 => {
                let node = a.get(term);
                let (x, f) = (node.kids[0], node.kids[1]);
                if !node.eval_flag {
                    // Lazy: the action has been evaluated to `return v`; now evaluate v itself.
                    // This is where `put`, `get` and `random` actually run.
                    if a.tag(x) != Tag::Unit {
                        return Err(RuntimeError::BindNotIo(self.print(x, ERROR_PRINT_LIMIT)));
                    }
                    let inner = a.kid(x, 0);
                    let node = a.get_mut(term);
                    node.kids[0] = inner;
                    node.eval_flag = true;
                    node.eval_state = 0;
                    push(a, ev);
                } else {
                    ev.term = a.node2(Tag::App, f, x);
                    rebind(a, ev);
                }
            }
            K::Thunk1 => {
                ev.term = a.kid(term, 0);
                rebind(a, ev);
            }
            K::Par => {
                let (spark, cont) = (a.kid(term, 0), a.kid(term, 1));
                ev.term = cont;
                rebind(a, ev);
                let id = self.next_eval_id;
                self.next_eval_id += 1;
                self.workers_spawned += 1;
                self.evals.push(Evaluator::new(id, spark));
            }
        }
        Ok(true)
    }

    fn number_arg(&self, ix: usize, op: Op, k: usize) -> Result<f64, RuntimeError> {
        let a = &self.arena;
        let x = a.kid(self.evals[ix].term, k);
        let node = a.get(x);
        if node.tag == Tag::Num {
            Ok(node.num)
        } else {
            Err(RuntimeError::NotANumber { op, term: self.print(x, ERROR_PRINT_LIMIT) })
        }
    }

    fn replace_focus_with_num(&mut self, ix: usize, n: f64) {
        let ev = &mut self.evals[ix];
        ev.term = self.arena.num(n);
        rebind(&mut self.arena, ev);
    }
}

/// Moves the focus to the child the focus is working on (`kids[eval_state]`), whose own
/// `eval_state` is reset (it may be a shared node evaluated before).
fn push(a: &mut Arena, ev: &mut Evaluator) {
    ev.stack.push(ev.term);
    let es = a.get(ev.term).eval_state as usize;
    ev.term = a.kid(ev.term, es);
    a.get_mut(ev.term).eval_state = 0;
}

/// Returns the (evaluated) focus to its parent. At the root, the evaluator is done.
fn pop(a: &mut Arena, ev: &mut Evaluator) -> bool {
    let Some(p) = ev.stack.pop() else {
        ev.done = true;
        return false;
    };
    let parent = a.get_mut(p);
    parent.kids[parent.eval_state as usize] = ev.term;
    match parent.tag {
        // Both halves of a cons are done at once (see the `cons1` quirk).
        Tag::Cons => parent.eval_state = 2,
        Tag::Thunk => {
            parent.eval_flag = true;
            parent.eval_state += 1;
        }
        _ => parent.eval_state += 1,
    }
    ev.term = p;
    true
}

/// Writes a rewritten focus back into the parent's slot.
fn rebind(a: &mut Arena, ev: &Evaluator) {
    if let Some(&p) = ev.stack.last() {
        let parent = a.get_mut(p);
        parent.kids[parent.eval_state as usize] = ev.term;
    }
}
