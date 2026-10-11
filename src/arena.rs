//! The term graph. Terms are mutable graphs with sharing and cycles (lazy thunks, `Y`), and the
//! machine overwrites child slots in place, so nodes live in an index-based arena.

use std::collections::HashMap;

use crate::prim::Op;

pub type NodeId = u32;
pub const NIL: NodeId = NodeId::MAX;

/// Interned variable name.
pub type Sym = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Var,
    Abs,
    Rec,
    /// Application whose argument is not evaluated first (cbn, lazy).
    App,
    /// Strict application (cbv): function, then argument, are evaluated before substitution.
    Aps,
    Num,
    Op0,
    Op1,
    Op2,
    If,
    Cons,
    Car,
    Cdr,
    IsCons,
    Bind,
    Unit,
    Thunk,
    Par,
    /// A slot on the free list.
    Free,
}

impl Tag {
    pub fn arity(self) -> usize {
        match self {
            Tag::Var | Tag::Num | Tag::Op0 | Tag::Free => 0,
            Tag::Abs | Tag::Rec | Tag::Op1 | Tag::Car | Tag::Cdr | Tag::IsCons | Tag::Unit | Tag::Thunk => 1,
            Tag::App | Tag::Aps | Tag::Op2 | Tag::Cons | Tag::Bind | Tag::Par => 2,
            Tag::If => 3,
        }
    }
}

/// One term node. Mirrors the JS objects: `kids` is the JS `nodes[]` array and `eval_state` says
/// which child the machine evaluates next.
#[derive(Clone, Debug)]
pub struct Node {
    pub tag: Tag,
    pub kids: [NodeId; 3],
    pub eval_state: u8,
    /// `Bind`: the first argument is already unwrapped from its `Unit` (always true outside lazy).
    /// `Thunk`: the value has been computed.
    pub eval_flag: bool,
    /// `Abs`: substitute by sharing (lazy) instead of copying.
    pub lazy: bool,
    /// `Thunk`: created by substitution or `Y`, so it is shared and never copied.
    pub closed: bool,
    /// `Thunk`: id of the evaluator that started evaluating it (the JS `open` flag).
    pub open_by: Option<u32>,
    pub name: Sym,
    pub num: f64,
    pub op: Op,
}

impl Node {
    fn new(tag: Tag) -> Node {
        Node {
            tag,
            kids: [NIL; 3],
            eval_state: 0,
            eval_flag: false,
            lazy: false,
            closed: false,
            open_by: None,
            name: 0,
            num: 0.0,
            op: Op::Get,
        }
    }
}

#[derive(Default)]
pub struct Interner {
    names: Vec<String>,
    ids: HashMap<String, Sym>,
}

impl Interner {
    pub fn intern(&mut self, s: &str) -> Sym {
        if let Some(&id) = self.ids.get(s) {
            return id;
        }
        let id = self.names.len() as Sym;
        self.names.push(s.to_string());
        self.ids.insert(s.to_string(), id);
        id
    }

    pub fn name(&self, sym: Sym) -> &str {
        &self.names[sym as usize]
    }
}

pub struct Arena {
    nodes: Vec<Node>,
    free: Vec<NodeId>,
    pub syms: Interner,
    allocs_since_gc: usize,
    live_after_gc: usize,
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    pub fn new() -> Arena {
        Arena { nodes: Vec::new(), free: Vec::new(), syms: Interner::default(), allocs_since_gc: 0, live_after_gc: 0 }
    }

    #[inline]
    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    #[inline]
    pub fn get_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id as usize]
    }

    #[inline]
    pub fn tag(&self, id: NodeId) -> Tag {
        self.nodes[id as usize].tag
    }

    #[inline]
    pub fn kid(&self, id: NodeId, i: usize) -> NodeId {
        self.nodes[id as usize].kids[i]
    }

    pub fn alloc(&mut self, node: Node) -> NodeId {
        self.allocs_since_gc += 1;
        if let Some(id) = self.free.pop() {
            self.nodes[id as usize] = node;
            id
        } else {
            assert!(self.nodes.len() < NIL as usize, "term graph too large");
            self.nodes.push(node);
            (self.nodes.len() - 1) as NodeId
        }
    }

    // --- constructors (fresh nodes always start with eval_state 0, like `new LamX(...)`) ---

    pub fn var(&mut self, name: Sym) -> NodeId {
        self.alloc(Node { name, ..Node::new(Tag::Var) })
    }

    pub fn abs(&mut self, lazy: bool, name: Sym, body: NodeId) -> NodeId {
        self.alloc(Node { lazy, name, kids: [body, NIL, NIL], ..Node::new(Tag::Abs) })
    }

    pub fn rec(&mut self, name: Sym, body: NodeId) -> NodeId {
        self.alloc(Node { name, kids: [body, NIL, NIL], ..Node::new(Tag::Rec) })
    }

    pub fn num(&mut self, num: f64) -> NodeId {
        self.alloc(Node { num, ..Node::new(Tag::Num) })
    }

    pub fn op0(&mut self, op: Op) -> NodeId {
        self.alloc(Node { op, ..Node::new(Tag::Op0) })
    }

    pub fn op1(&mut self, op: Op, a: NodeId) -> NodeId {
        self.alloc(Node { op, kids: [a, NIL, NIL], ..Node::new(Tag::Op1) })
    }

    pub fn op2(&mut self, op: Op, a: NodeId, b: NodeId) -> NodeId {
        self.alloc(Node { op, kids: [a, b, NIL], ..Node::new(Tag::Op2) })
    }

    pub fn node1(&mut self, tag: Tag, a: NodeId) -> NodeId {
        self.alloc(Node { kids: [a, NIL, NIL], ..Node::new(tag) })
    }

    pub fn node2(&mut self, tag: Tag, a: NodeId, b: NodeId) -> NodeId {
        self.alloc(Node { kids: [a, b, NIL], ..Node::new(tag) })
    }

    pub fn node3(&mut self, tag: Tag, a: NodeId, b: NodeId, c: NodeId) -> NodeId {
        self.alloc(Node { kids: [a, b, c], ..Node::new(tag) })
    }

    pub fn bind(&mut self, a: NodeId, b: NodeId, eval_flag: bool) -> NodeId {
        self.alloc(Node { eval_flag, kids: [a, b, NIL], ..Node::new(Tag::Bind) })
    }

    pub fn thunk(&mut self, closed: bool, body: NodeId) -> NodeId {
        self.alloc(Node { closed, kids: [body, NIL, NIL], ..Node::new(Tag::Thunk) })
    }

    // --- garbage collection ---

    pub fn live(&self) -> usize {
        self.nodes.len() - self.free.len()
    }

    /// True when enough has been allocated since the last collection to make one worthwhile.
    pub fn wants_gc(&self) -> bool {
        self.allocs_since_gc > self.live_after_gc.max(1 << 20)
    }

    /// Mark & sweep from `roots`. Every `NodeId` not reachable from them becomes invalid.
    pub fn collect(&mut self, roots: impl IntoIterator<Item = NodeId>) {
        let mut marked = vec![false; self.nodes.len()];
        let mut work: Vec<NodeId> = roots.into_iter().filter(|&r| r != NIL).collect();
        while let Some(id) = work.pop() {
            if std::mem::replace(&mut marked[id as usize], true) {
                continue;
            }
            let n = &self.nodes[id as usize];
            for &k in &n.kids[..n.tag.arity()] {
                if k != NIL && !marked[k as usize] {
                    work.push(k);
                }
            }
        }
        self.free.clear();
        for (i, n) in self.nodes.iter_mut().enumerate() {
            if !marked[i] {
                n.tag = Tag::Free;
                self.free.push(i as NodeId);
            }
        }
        // Hand out low ids first, which keeps the working set compact.
        self.free.reverse();
        self.allocs_since_gc = 0;
        self.live_after_gc = self.live();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gc_keeps_reachable_cycles_and_frees_garbage() {
        let mut a = Arena::new();
        let x = a.syms.intern("x");
        let garbage = a.var(x);
        let t = a.thunk(true, NIL);
        let body = a.node2(Tag::Cons, t, t);
        a.get_mut(t).kids[0] = body;
        a.collect([t]);
        assert_eq!(a.live(), 2);
        assert_eq!(a.tag(garbage), Tag::Free);
        assert_eq!(a.tag(a.kid(t, 0)), Tag::Cons);
        // The freed slot is reused.
        assert_eq!(a.num(1.0), garbage);
    }
}
