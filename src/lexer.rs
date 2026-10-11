//! Tokenizer. Reproduces the splitting of `LamParser.tokenize`: identifiers are runs of lowercase
//! letters, numbers are runs of digits and operator characters glue together (`<=`, but also `+-`).

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Backslash,
    Dot,
    LParen,
    RParen,
    At,
    Colon,
    /// Uppercase `Y` (recursion binder).
    Y,
    Num(String),
    /// A run of lowercase letters; keywords are recognised by the parser.
    Ident(String),
    /// A run of `+-*/%<=>^`.
    OpRun(String),
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Backslash => f.write_str("\\"),
            TokenKind::Dot => f.write_str("."),
            TokenKind::LParen => f.write_str("("),
            TokenKind::RParen => f.write_str(")"),
            TokenKind::At => f.write_str("@"),
            TokenKind::Colon => f.write_str(":"),
            TokenKind::Y => f.write_str("Y"),
            TokenKind::Num(s) | TokenKind::Ident(s) | TokenKind::OpRun(s) => f.write_str(s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("line {line}:{col}: unexpected character '{ch}'")]
pub struct LexError {
    pub ch: char,
    pub line: usize,
    pub col: usize,
}

fn is_op_char(c: char) -> bool {
    matches!(c, '+' | '-' | '*' | '/' | '%' | '<' | '=' | '>' | '^')
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = src.chars().collect();
    let mut tokens = Vec::new();
    let (mut line, mut col) = (1, 1);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let (tok_line, tok_col) = (line, col);
        let start = i;
        let kind = match c {
            '\n' => {
                i += 1;
                line += 1;
                col = 1;
                continue;
            }
            c if c.is_whitespace() => {
                i += 1;
                col += 1;
                continue;
            }
            '#' => {
                while i < chars.len() && chars[i] != '\n' && chars[i] != '\r' {
                    i += 1;
                }
                col += i - start;
                continue;
            }
            '\\' => TokenKind::Backslash,
            '.' => TokenKind::Dot,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '@' => TokenKind::At,
            ':' => TokenKind::Colon,
            'Y' => TokenKind::Y,
            c if c.is_ascii_digit() || c.is_ascii_lowercase() || is_op_char(c) => {
                let class = |d: char| {
                    if c.is_ascii_digit() {
                        d.is_ascii_digit()
                    } else if c.is_ascii_lowercase() {
                        d.is_ascii_lowercase()
                    } else {
                        is_op_char(d)
                    }
                };
                let mut j = i;
                while j < chars.len() && class(chars[j]) {
                    j += 1;
                }
                let text: String = chars[i..j].iter().collect();
                col += j - i;
                i = j;
                let kind = if c.is_ascii_digit() {
                    TokenKind::Num(text)
                } else if c.is_ascii_lowercase() {
                    TokenKind::Ident(text)
                } else {
                    TokenKind::OpRun(text)
                };
                tokens.push(Token { kind, line: tok_line, col: tok_col });
                continue;
            }
            ch => return Err(LexError { ch, line, col }),
        };
        i += 1;
        col += 1;
        tokens.push(Token { kind, line: tok_line, col: tok_col });
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<String> {
        tokenize(src).unwrap().iter().map(|t| t.kind.to_string()).collect()
    }

    #[test]
    fn splits_like_the_site() {
        assert_eq!(kinds("Yx.x"), ["Y", "x", ".", "x"]);
        assert_eq!(kinds("<= 1 2"), ["<=", "1", "2"]);
        assert_eq!(kinds("+-"), ["+-"]);
        assert_eq!(kinds("x1"), ["x", "1"]);
        assert_eq!(kinds("1.5"), ["1", ".", "5"]);
        assert_eq!(kinds("@@\\f.f"), ["@", "@", "\\", "f", ".", "f"]);
    }

    #[test]
    fn comments() {
        assert_eq!(kinds("# hello\n1 # two\n#\n3"), ["1", "3"]);
    }

    #[test]
    fn positions_and_errors() {
        let t = tokenize("a\n  bc").unwrap();
        assert_eq!((t[1].line, t[1].col), (2, 3));
        assert_eq!(tokenize("x\n $").unwrap_err(), LexError { ch: '$', line: 2, col: 2 });
    }
}
