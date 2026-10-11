//! Recursive-descent parser, a port of `LamParser.parseTerm`. Every construct is prefix with a fixed
//! number of arguments, so no precedence is involved. The strategy is chosen here: the same source
//! desugars into different graphs for lazy, cbn and cbv.

use std::fmt;

use crate::arena::{Arena, NodeId, Sym, Tag};
use crate::lexer::{LexError, Token, TokenKind, tokenize};
use crate::prim::Op;
use crate::stack;
use crate::strategy::Strategy;
use crate::subst;

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "let", "letrec", "in", "car", "cdr", "iscons", "bind", "return", "par", "put", "get",
    "alert", "random", "not", "log", "sqrt", "floor", "ceil", "round",
];

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub struct ParseError {
    pub message: String,
    /// Line and column, or `None` at the end of the input.
    pub pos: Option<(usize, usize)>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pos {
            Some((line, col)) => write!(f, "line {line}:{col}: {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        ParseError { message: format!("unexpected character '{}'", e.ch), pos: Some((e.line, e.col)) }
    }
}

/// Parses a whole program into `arena` and returns its root.
pub fn parse(src: &str, strategy: Strategy, arena: &mut Arena) -> Result<NodeId, ParseError> {
    let tokens = tokenize(src)?;
    let mut p = Parser { tokens, pos: 0, arena, strategy };
    let term = p.term()?;
    if let Some(tok) = p.tokens.get(p.pos) {
        return Err(p.error_at(tok, format!("unexpected token '{}' after end of program", tok.kind)));
    }
    Ok(term)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    arena: &'a mut Arena,
    strategy: Strategy,
}

impl Parser<'_> {
    fn lazy(&self) -> bool {
        self.strategy == Strategy::Lazy
    }

    fn error_at(&self, tok: &Token, message: String) -> ParseError {
        ParseError { message, pos: Some((tok.line, tok.col)) }
    }

    fn next(&mut self, expected: &str) -> Result<Token, ParseError> {
        match self.tokens.get(self.pos) {
            Some(tok) => {
                self.pos += 1;
                Ok(tok.clone())
            }
            None => Err(ParseError { message: format!("unexpected end of input, expected {expected}"), pos: None }),
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        let what = format!("'{kind}'");
        let tok = self.next(&what)?;
        if tok.kind == kind {
            Ok(())
        } else {
            Err(self.error_at(&tok, format!("expected {what}, found '{}'", tok.kind)))
        }
    }

    fn expect_keyword(&mut self, word: &str) -> Result<(), ParseError> {
        self.expect(TokenKind::Ident(word.to_string()))
    }

    /// A binder name. Unlike the site, keywords are rejected: such a variable could never be used.
    fn binder(&mut self) -> Result<Sym, ParseError> {
        let tok = self.next("a variable name")?;
        match &tok.kind {
            TokenKind::Ident(name) if !KEYWORDS.contains(&name.as_str()) => Ok(self.arena.syms.intern(name)),
            TokenKind::Ident(name) => {
                Err(self.error_at(&tok, format!("keyword '{name}' cannot be used as a variable name")))
            }
            other => Err(self.error_at(&tok, format!("expected a variable name, found '{other}'"))),
        }
    }

    fn app(&mut self, f: NodeId, a: NodeId) -> NodeId {
        let tag = if self.strategy == Strategy::ByValue { Tag::Aps } else { Tag::App };
        self.arena.node2(tag, f, a)
    }

    /// IO actions are wrapped in `return` (Unit) under lazy evaluation and are plain terms otherwise.
    fn io(&mut self, t: NodeId) -> NodeId {
        if self.lazy() { self.arena.node1(Tag::Unit, t) } else { t }
    }

    /// One term. Nesting depth is only limited by memory (see [`stack::grow`]).
    fn term(&mut self) -> Result<NodeId, ParseError> {
        stack::grow(|| self.term_unguarded())
    }

    fn term_unguarded(&mut self) -> Result<NodeId, ParseError> {
        let tok = self.next("a term")?;
        let lazy = self.lazy();
        let word = match &tok.kind {
            TokenKind::Ident(w) => w.clone(),
            TokenKind::LParen => {
                let e = self.term()?;
                self.expect(TokenKind::RParen)?;
                return Ok(e);
            }
            TokenKind::Backslash => {
                let name = self.binder()?;
                self.expect(TokenKind::Dot)?;
                let body = self.term()?;
                return Ok(self.arena.abs(lazy, name, body));
            }
            TokenKind::Y => {
                let name = self.binder()?;
                self.expect(TokenKind::Dot)?;
                let body = self.term()?;
                return Ok(self.arena.rec(name, body));
            }
            TokenKind::At => {
                let f = self.term()?;
                let a = self.term()?;
                return Ok(self.app(f, a));
            }
            TokenKind::Colon => {
                let head = self.term()?;
                let tail = self.term()?;
                return Ok(self.cons(head, tail));
            }
            TokenKind::Num(digits) => {
                let n: f64 = digits.parse().expect("digit run is a valid number");
                return Ok(self.arena.num(n));
            }
            TokenKind::OpRun(s) => {
                let Some(op) = Op::binary(s) else {
                    return Err(self.error_at(&tok, format!("unknown operator '{s}'")));
                };
                let a = self.term()?;
                let b = self.term()?;
                return Ok(self.arena.op2(op, a, b));
            }
            TokenKind::Dot | TokenKind::RParen => {
                return Err(self.error_at(&tok, format!("expected a term, found '{}'", tok.kind)));
            }
        };

        Ok(match word.as_str() {
            "if" => {
                let c = self.term()?;
                self.expect_keyword("then")?;
                let t = self.term()?;
                self.expect_keyword("else")?;
                let e = self.term()?;
                self.arena.node3(Tag::If, c, t, e)
            }
            "car" | "cdr" | "iscons" => {
                let tag = match word.as_str() {
                    "car" => Tag::Car,
                    "cdr" => Tag::Cdr,
                    _ => Tag::IsCons,
                };
                let e = self.term()?;
                self.arena.node1(tag, e)
            }
            "bind" => {
                let a = self.term()?;
                let b = self.term()?;
                // Outside lazy mode there is no Unit to unwrap.
                self.arena.bind(a, b, !lazy)
            }
            "return" => {
                let e = self.term()?;
                self.io(e)
            }
            "par" => {
                if !lazy {
                    return Err(self
                        .error_at(&tok, "par is only allowed for lazy evaluation (use --strategy lazy)".to_string()));
                }
                let a = self.term()?;
                let b = self.term()?;
                self.arena.node2(Tag::Par, a, b)
            }
            "get" | "random" => {
                let op = if word == "get" { Op::Get } else { Op::Random };
                let e = self.arena.op0(op);
                self.io(e)
            }
            "put" | "alert" => {
                let op = if word == "put" { Op::Put } else { Op::Alert };
                let arg = self.term()?;
                let e = self.arena.op1(op, arg);
                self.io(e)
            }
            "let" => {
                let name = self.binder()?;
                self.expect(TokenKind::OpRun("=".to_string()))?;
                let value = self.term()?;
                self.expect_keyword("in")?;
                let body = self.term()?;
                let f = self.arena.abs(lazy, name, body);
                self.app(f, value)
            }
            "letrec" => {
                let name = self.binder()?;
                self.expect(TokenKind::OpRun("=".to_string()))?;
                let value = self.term()?;
                self.expect_keyword("in")?;
                let body = self.term()?;
                self.letrec(name, value, body)
            }
            w => {
                if let Some(op) = Op::unary(w) {
                    let e = self.term()?;
                    self.arena.op1(op, e)
                } else if KEYWORDS.contains(&w) {
                    return Err(self.error_at(&tok, format!("expected a term, found keyword '{w}'")));
                } else {
                    let sym = self.arena.syms.intern(w);
                    self.arena.var(sym)
                }
            }
        })
    }

    fn cons(&mut self, head: NodeId, tail: NodeId) -> NodeId {
        match self.strategy {
            Strategy::Lazy => {
                let h = self.arena.thunk(false, head);
                let t = self.arena.thunk(false, tail);
                self.arena.node2(Tag::Cons, h, t)
            }
            // `@! @! (\x.\y.: x y) head tail`, so that both halves are evaluated.
            Strategy::ByValue => {
                let x = self.arena.syms.intern("x");
                let y = self.arena.syms.intern("y");
                let vx = self.arena.var(x);
                let vy = self.arena.var(y);
                let c = self.arena.node2(Tag::Cons, vx, vy);
                let ly = self.arena.abs(false, y, c);
                let lx = self.arena.abs(false, x, ly);
                let a1 = self.arena.node2(Tag::Aps, lx, head);
                self.arena.node2(Tag::Aps, a1, tail)
            }
            Strategy::ByName => self.arena.node2(Tag::Cons, head, tail),
        }
    }

    fn letrec(&mut self, name: Sym, value: NodeId, body: NodeId) -> NodeId {
        if self.lazy() {
            let f = self.arena.abs(true, name, body);
            let r = self.arena.rec(name, value);
            return self.arena.node2(Tag::App, f, r);
        }
        // cbn/cbv: self-application. `letrec f = A in B` becomes
        // `@ (\f.B) (@ (\x.@ x x) (\f.A[f := @ f f]))`.
        let tag = if self.strategy == Strategy::ByValue { Tag::Aps } else { Tag::App };
        let x = self.arena.syms.intern("x");
        let vf1 = self.arena.var(name);
        let vf2 = self.arena.var(name);
        let ff = self.arena.node2(tag, vf1, vf2);
        let value = subst::substitute_clone(self.arena, value, name, ff);
        let gen_ = self.arena.abs(false, name, value);
        let vx1 = self.arena.var(x);
        let vx2 = self.arena.var(x);
        let xx = self.arena.node2(tag, vx1, vx2);
        let self_app = self.arena.abs(false, x, xx);
        let fix = self.arena.node2(tag, self_app, gen_);
        let f = self.arena.abs(false, name, body);
        self.arena.node2(tag, f, fix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::printer::{PrintOptions, print_term};

    fn show(src: &str, s: Strategy) -> String {
        let mut a = Arena::new();
        let root = parse(src, s, &mut a).unwrap();
        print_term(&a, root, &PrintOptions { limit: 1000, trace: true, focus: None })
    }

    fn err(src: &str, s: Strategy) -> String {
        parse(src, s, &mut Arena::new()).unwrap_err().to_string()
    }

    #[test]
    fn desugaring_per_strategy() {
        use Strategy::*;
        assert_eq!(show("@ f a", ByName), "@ f a");
        assert_eq!(show("@ f a", ByValue), "@! f a");
        assert_eq!(show(": 1 2", Lazy), ": [1] [2]");
        assert_eq!(show(": 1 2", ByName), ": 1 2");
        assert_eq!(show(": 1 2", ByValue), "@! @! \\x. \\y. : x y 1 2");
        assert_eq!(show("let x = 1 in x", Lazy), "@ \\x. x 1");
        assert_eq!(show("letrec f = f in f", Lazy), "@ \\f. f Y f. f");
        assert_eq!(show("letrec f = f in f", ByName), "@ \\f. f @ \\x. @ x x \\f. @ f f");
        assert_eq!(show("letrec f = f in f", ByValue), "@! \\f. f @! \\x. @! x x \\f. @! f f");
        assert_eq!(show("return 1", Lazy), "return 1");
        assert_eq!(show("return 1", ByName), "1");
        assert_eq!(show("put 1", Lazy), "return put 1");
        assert_eq!(show("put 1", ByValue), "put 1");
        assert_eq!(show("get", Lazy), "return get");
        assert_eq!(show("random", ByName), "random");
        assert_eq!(show("bind get \\x.x", ByName), "bind get \\x. x");
        assert_eq!(show("par 1 2", Lazy), "par 1 2");
        assert_eq!(show("Yx.x", ByName), "Y x. x");
        assert_eq!(show("if <= 1 2 then (1) else not 0", ByName), "if <= 1 2 then 1 else not 0");
    }

    #[test]
    fn bind_flag_depends_on_strategy() {
        let mut a = Arena::new();
        let r = parse("bind 1 2", Strategy::Lazy, &mut a).unwrap();
        assert!(!a.get(r).eval_flag);
        let r = parse("bind 1 2", Strategy::ByName, &mut a).unwrap();
        assert!(a.get(r).eval_flag);
    }

    #[test]
    fn errors() {
        use Strategy::*;
        assert_eq!(err("par 1 2", ByName), "line 1:1: par is only allowed for lazy evaluation (use --strategy lazy)");
        assert_eq!(err("if 1 else 2", Lazy), "line 1:6: expected 'then', found 'else'");
        assert_eq!(err("1 2", Lazy), "line 1:3: unexpected token '2' after end of program");
        assert_eq!(err("+ 1", Lazy), "unexpected end of input, expected a term");
        assert_eq!(err("+- 1 2", Lazy), "line 1:1: unknown operator '+-'");
        assert_eq!(err("\\if.1", Lazy), "line 1:2: keyword 'if' cannot be used as a variable name");
        assert_eq!(err("x\n $", Lazy), "line 2:2: unexpected character '$'");
    }

    #[test]
    fn all_examples_parse() {
        for ex in crate::examples::EXAMPLES {
            for s in [Strategy::Lazy, Strategy::ByName, Strategy::ByValue] {
                let r = parse(ex.source, s, &mut Arena::new());
                let uses_par = (10..=14).contains(&ex.number);
                if s != Strategy::Lazy && uses_par {
                    assert!(r.unwrap_err().message.starts_with("par is only allowed"), "{}", ex.slug);
                } else {
                    assert!(r.is_ok(), "example {} ({s}): {:?}", ex.slug, r.err());
                }
            }
        }
    }
}
