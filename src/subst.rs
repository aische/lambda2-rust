//! Substitution, ported node kind by node kind from `LamTypes.js`. Which nodes are shared and
//! which are copied matters: it decides what gets memoised, and therefore the step counts.
//!
//! These functions recurse over the term; the interpreter runs on a thread with a large stack.

use crate::arena::{Arena, NIL, Node, NodeId, Sym, Tag};

/// A fresh node like `n` (same kind and payload) with the given children, `eval_state` 0 and not
/// opened by anyone, like the JS `new LamX(...)` calls.
fn fresh_like(a: &mut Arena, n: NodeId, kids: [NodeId; 3]) -> NodeId {
    let src = a.get(n);
    let node = Node { kids, eval_state: 0, open_by: None, ..src.clone() };
    a.alloc(node)
}

fn kids_of(a: &Arena, n: NodeId) -> ([NodeId; 3], usize) {
    let node = a.get(n);
    (node.kids, node.tag.arity())
}

/// Lazy-mode substitution (`LamX.prototype.substitute`). Returns `t` itself at every occurrence
/// (sharing), never enters closed thunks and copies open ones. Not capture-avoiding: the lazy
/// machine only ever substitutes closed terms.
pub fn substitute(a: &mut Arena, n: NodeId, name: Sym, t: NodeId) -> NodeId {
    let node = a.get(n);
    match node.tag {
        Tag::Var => {
            if node.name == name {
                t
            } else {
                fresh_like(a, n, [NIL; 3])
            }
        }
        Tag::Abs | Tag::Rec if node.name == name => n,
        Tag::Op0 => n,
        Tag::Thunk if node.closed => n,
        _ => {
            let (mut kids, arity) = kids_of(a, n);
            for k in &mut kids[..arity] {
                *k = substitute(a, *k, name, t);
            }
            fresh_like(a, n, kids)
        }
    }
}

/// Deep copy (`LamX.prototype.clone`). Closed thunks are shared (the JS has no `clone` for thunks
/// and crashes; sharing is what keeps cyclic `Y` graphs finite).
pub fn clone(a: &mut Arena, n: NodeId) -> NodeId {
    if n == NIL {
        return NIL;
    }
    let node = a.get(n);
    if node.tag == Tag::Thunk && node.closed {
        return n;
    }
    let (mut kids, arity) = kids_of(a, n);
    for k in &mut kids[..arity] {
        *k = clone(a, *k);
    }
    fresh_like(a, n, kids)
}

/// `substituteClone`: like [`substitute`], but every occurrence gets its own copy of `t`.
/// Used by the cbn/cbv `letrec` encoding and for alpha-renaming.
pub fn substitute_clone(a: &mut Arena, n: NodeId, name: Sym, t: NodeId) -> NodeId {
    let node = a.get(n);
    match node.tag {
        Tag::Var if node.name == name => clone(a, t),
        Tag::Abs | Tag::Rec if node.name == name => clone(a, n),
        Tag::Thunk if node.closed => n,
        _ => {
            let (mut kids, arity) = kids_of(a, n);
            for k in &mut kids[..arity] {
                *k = substitute_clone(a, *k, name, t);
            }
            fresh_like(a, n, kids)
        }
    }
}

/// `substituteCloneAlpha`: the cbn/cbv substitution. Copies everything and renames a binder `y` to
/// `y_<n>` when `y` is free in `t`, so `t` cannot be captured.
pub fn substitute_clone_alpha(
    a: &mut Arena,
    n: NodeId,
    name: Sym,
    t: NodeId,
    free: &[Sym],
    next_id: &mut u32,
) -> NodeId {
    let node = a.get(n);
    match node.tag {
        Tag::Var if node.name == name => clone(a, t),
        Tag::Abs | Tag::Rec if node.name == name => clone(a, n),
        Tag::Abs | Tag::Rec if free.contains(&node.name) => {
            let old = node.name;
            let body = node.kids[0];
            let new_name = format!("{}_{}", a.syms.name(old), *next_id);
            *next_id += 1;
            let new_sym = a.syms.intern(&new_name);
            let new_var = a.var(new_sym);
            let renamed = substitute_clone(a, body, old, new_var);
            let body = substitute_clone_alpha(a, renamed, name, t, free, next_id);
            let r = fresh_like(a, n, [body, NIL, NIL]);
            a.get_mut(r).name = new_sym;
            r
        }
        Tag::Thunk if node.closed => n,
        _ => {
            let (mut kids, arity) = kids_of(a, n);
            for k in &mut kids[..arity] {
                *k = substitute_clone_alpha(a, *k, name, t, free, next_id);
            }
            fresh_like(a, n, kids)
        }
    }
}

/// Free variables of `n`, in order of first occurrence. Closed thunks are closed terms (and may be
/// cyclic), so they are not entered.
pub fn free_vars(a: &Arena, n: NodeId) -> Vec<Sym> {
    fn go(a: &Arena, n: NodeId, bound: &mut Vec<Sym>, free: &mut Vec<Sym>) {
        if n == NIL {
            return;
        }
        let node = a.get(n);
        match node.tag {
            Tag::Var => {
                if !bound.contains(&node.name) && !free.contains(&node.name) {
                    free.push(node.name);
                }
            }
            Tag::Abs | Tag::Rec => {
                bound.push(node.name);
                go(a, node.kids[0], bound, free);
                bound.pop();
            }
            Tag::Thunk if node.closed => {}
            tag => {
                for &k in &node.kids[..tag.arity()] {
                    go(a, k, bound, free);
                }
            }
        }
    }
    let mut free = Vec::new();
    go(a, n, &mut Vec::new(), &mut free);
    free
}
