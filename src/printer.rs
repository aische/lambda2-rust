//! Term → source text. Every construct is prefix, so no parentheses are needed. The printer is
//! iterative (long lists cannot overflow the stack), cycle-safe and size-limited.
//!
//! `tools/oracle/run.js` contains a JS copy of the non-trace format; keep the two in sync.

use std::collections::HashSet;

use crate::arena::{Arena, NIL, NodeId, Tag};
use crate::prim::format_js;

pub struct PrintOptions<'a> {
    /// Maximum number of nodes to print before cutting off with `…`.
    pub limit: usize,
    /// Show thunks as `[…]` and strict applications as `@!`.
    pub trace: bool,
    /// Path from the root to the focus (an evaluator's stack plus its focus). The focus is
    /// wrapped in `⟨ ⟩`.
    pub focus: Option<&'a [NodeId]>,
}

impl Default for PrintOptions<'_> {
    fn default() -> Self {
        PrintOptions { limit: 2000, trace: false, focus: None }
    }
}

enum Item {
    /// A node, with its index in the focus path if it lies on that path.
    Node(NodeId, Option<usize>),
    Word(String),
    /// Leaves a thunk (cycle detection); `bool`: close a trace bracket.
    LeaveThunk(NodeId, bool),
    CloseFocus,
}

struct Out {
    s: String,
    space: bool,
}

impl Out {
    fn word(&mut self, w: &str) {
        if self.space {
            self.s.push(' ');
        }
        self.s.push_str(w);
        self.space = true;
    }

    /// An opening bracket glued to what follows.
    fn open(&mut self, w: &str) {
        self.word(w);
        self.space = false;
    }

    /// A closing bracket glued to what precedes.
    fn close(&mut self, w: &str) {
        self.s.push_str(w);
        self.space = true;
    }
}

pub fn print_term(a: &Arena, root: NodeId, opts: &PrintOptions) -> String {
    let mut out = Out { s: String::new(), space: false };
    let mut on_path: HashSet<NodeId> = HashSet::new();
    let mut count = 0;
    let focus = opts.focus.unwrap_or(&[]);
    let mut work = vec![Item::Node(root, if focus.first() == Some(&root) { Some(0) } else { None })];

    while let Some(item) = work.pop() {
        let (id, path_ix) = match item {
            Item::Word(w) => {
                out.word(&w);
                continue;
            }
            Item::LeaveThunk(id, bracket) => {
                on_path.remove(&id);
                if bracket {
                    out.close("]");
                }
                continue;
            }
            Item::CloseFocus => {
                out.close("⟩");
                continue;
            }
            Item::Node(id, path_ix) => (id, path_ix),
        };
        if id == NIL {
            out.word("?");
            continue;
        }
        let n = a.get(id);
        let is_focus = path_ix.is_some_and(|i| i + 1 == focus.len());
        // Index in the focus path of the child the machine is working on.
        let next_ix = path_ix.filter(|_| !is_focus).map(|i| i + 1);
        let kid_ix = |k: usize| next_ix.filter(|&i| focus[i] == n.kids[k] && n.eval_state as usize == k);

        if n.tag == Tag::Thunk {
            if on_path.contains(&id) {
                out.word("…");
                continue;
            }
            on_path.insert(id);
            if is_focus {
                out.open("⟨");
                work.push(Item::CloseFocus);
            }
            if opts.trace {
                out.open("[");
            }
            work.push(Item::LeaveThunk(id, opts.trace));
            work.push(Item::Node(n.kids[0], kid_ix(0)));
            continue;
        }

        count += 1;
        if count > opts.limit {
            out.word("…");
            break;
        }
        if is_focus {
            out.open("⟨");
            work.push(Item::CloseFocus);
        }
        let head = match n.tag {
            Tag::Var => a.syms.name(n.name).to_string(),
            Tag::Abs => format!("\\{}.", a.syms.name(n.name)),
            Tag::Rec => format!("Y {}.", a.syms.name(n.name)),
            Tag::App => "@".to_string(),
            Tag::Aps => if opts.trace { "@!" } else { "@" }.to_string(),
            Tag::Num => format_js(n.num),
            Tag::Op0 | Tag::Op1 | Tag::Op2 => n.op.name().to_string(),
            Tag::If => "if".to_string(),
            Tag::Cons => ":".to_string(),
            Tag::Car => "car".to_string(),
            Tag::Cdr => "cdr".to_string(),
            Tag::IsCons => "iscons".to_string(),
            Tag::Bind => "bind".to_string(),
            Tag::Unit => "return".to_string(),
            Tag::Par => "par".to_string(),
            Tag::Free => "<freed>".to_string(),
            Tag::Thunk => unreachable!(),
        };
        out.word(&head);
        if n.tag == Tag::If {
            work.push(Item::Node(n.kids[2], kid_ix(2)));
            work.push(Item::Word("else".to_string()));
            work.push(Item::Node(n.kids[1], kid_ix(1)));
            work.push(Item::Word("then".to_string()));
            work.push(Item::Node(n.kids[0], kid_ix(0)));
        } else {
            for k in (0..n.tag.arity()).rev() {
                work.push(Item::Node(n.kids[k], kid_ix(k)));
            }
        }
    }
    out.s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;

    #[test]
    fn cyclic_thunks_print_an_ellipsis() {
        let mut a = Arena::new();
        let t = a.thunk(true, NIL);
        let one = a.num(1.0);
        let c = a.node2(Tag::Cons, one, t);
        a.get_mut(t).kids[0] = c;
        assert_eq!(print_term(&a, t, &PrintOptions::default()), ": 1 …");
        let trace = PrintOptions { trace: true, ..PrintOptions::default() };
        assert_eq!(print_term(&a, t, &trace), "[: 1 …]");
    }

    #[test]
    fn size_limit() {
        let mut a = Arena::new();
        let mut list = a.num(0.0);
        for i in 0..100 {
            let n = a.num(i as f64);
            list = a.node2(Tag::Cons, n, list);
        }
        let opts = PrintOptions { limit: 5, ..PrintOptions::default() };
        assert_eq!(print_term(&a, list, &opts), ": 99 : 98 : …");
        // Deep lists do not overflow the stack.
        for i in 0..200_000 {
            let n = a.num(i as f64);
            list = a.node2(Tag::Cons, n, list);
        }
        assert!(print_term(&a, list, &PrintOptions { limit: usize::MAX, ..PrintOptions::default() }).len() > 1_000_000);
    }

    #[test]
    fn focus_marker_follows_the_path() {
        let mut a = Arena::new();
        let x = a.syms.intern("x");
        let one = a.num(1.0);
        let vx = a.var(x);
        let add = a.op2(crate::prim::Op::Add, vx, one);
        a.get_mut(add).eval_state = 1;
        let opts = PrintOptions { focus: Some(&[add, one]), ..PrintOptions::default() };
        assert_eq!(print_term(&a, add, &opts), "+ x ⟨1⟩");
        let opts = PrintOptions { focus: Some(&[add]), ..PrintOptions::default() };
        assert_eq!(print_term(&a, add, &opts), "⟨+ x 1⟩");
    }
}
