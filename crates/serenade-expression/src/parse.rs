//! Lexer and recursive-descent parser.

use crate::ExpressionError;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Bool(bool),
    Int(i64),
    Str(String),
    Var(String),
    Unary {
        op: UnaryOp,
        expr: Box<Self>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Self>,
        right: Box<Self>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Bool(bool),
    Int(i64),
    Str(String),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    LParen,
    RParen,
    Dot,
}

/// Parse `source` into an AST.
pub fn parse(source: &str) -> Result<Expr, ExpressionError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
    };
    let expr = parser.parse_or()?;
    if parser.peek().is_some() {
        return Err(ExpressionError::Parse {
            message: "unexpected trailing tokens".to_owned(),
        });
    }
    Ok(expr)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(tok)
    }

    fn parse_or(&mut self) -> Result<Expr, ExpressionError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Some(Token::OrOr)) {
            self.bump();
            let right = self.parse_and()?;
            left = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ExpressionError> {
        let mut left = self.parse_equality()?;
        while matches!(self.peek(), Some(Token::AndAnd)) {
            self.bump();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ExpressionError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Some(Token::EqEq) => BinaryOp::Eq,
                Some(Token::NotEq) => BinaryOp::Ne,
                _ => break,
            };
            self.bump();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ExpressionError> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.peek() {
                Some(Token::Lt) => BinaryOp::Lt,
                Some(Token::Le) => BinaryOp::Le,
                Some(Token::Gt) => BinaryOp::Gt,
                Some(Token::Ge) => BinaryOp::Ge,
                _ => break,
            };
            self.bump();
            let right = self.parse_term()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, ExpressionError> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinaryOp::Add,
                Some(Token::Minus) => BinaryOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_factor()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, ExpressionError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinaryOp::Mul,
                Some(Token::Slash) => BinaryOp::Div,
                _ => break,
            };
            self.bump();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ExpressionError> {
        match self.peek() {
            Some(Token::Bang) => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Some(Token::Minus) => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ExpressionError> {
        match self.bump().cloned() {
            Some(Token::Bool(v)) => Ok(Expr::Bool(v)),
            Some(Token::Int(v)) => Ok(Expr::Int(v)),
            Some(Token::Str(v)) => Ok(Expr::Str(v)),
            Some(Token::Ident(name)) => self.parse_path(name),
            Some(Token::LParen) => {
                let expr = self.parse_or()?;
                match self.bump() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err(ExpressionError::Parse {
                        message: "expected ')'".to_owned(),
                    }),
                }
            }
            Some(_) | None => Err(ExpressionError::Parse {
                message: "expected expression".to_owned(),
            }),
        }
    }

    fn parse_path(&mut self, first: String) -> Result<Expr, ExpressionError> {
        let mut path = first;
        while matches!(self.peek(), Some(Token::Dot)) {
            self.bump();
            match self.bump().cloned() {
                Some(Token::Ident(part)) => {
                    path.push('.');
                    path.push_str(&part);
                }
                _ => {
                    return Err(ExpressionError::Parse {
                        message: "expected identifier after '.'".to_owned(),
                    });
                }
            }
        }
        Ok(Expr::Var(path))
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, ExpressionError> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b == b'"' {
            let (s, next) = read_string(source, i)?;
            tokens.push(Token::Str(s));
            i = next;
            continue;
        }
        if b.is_ascii_digit() {
            let (n, next) = read_int(source, i)?;
            tokens.push(Token::Int(n));
            i = next;
            continue;
        }
        if is_ident_start(b) {
            let (ident, next) = read_ident(source, i);
            tokens.push(match ident.as_str() {
                "true" => Token::Bool(true),
                "false" => Token::Bool(false),
                _ => Token::Ident(ident),
            });
            i = next;
            continue;
        }
        let two = source.get(i..i + 2).unwrap_or("");
        let (tok, advance) = match two {
            "==" => (Token::EqEq, 2),
            "!=" => (Token::NotEq, 2),
            "<=" => (Token::Le, 2),
            ">=" => (Token::Ge, 2),
            "&&" => (Token::AndAnd, 2),
            "||" => (Token::OrOr, 2),
            _ => match b {
                b'+' => (Token::Plus, 1),
                b'-' => (Token::Minus, 1),
                b'*' => (Token::Star, 1),
                b'/' => (Token::Slash, 1),
                b'<' => (Token::Lt, 1),
                b'>' => (Token::Gt, 1),
                b'!' => (Token::Bang, 1),
                b'(' => (Token::LParen, 1),
                b')' => (Token::RParen, 1),
                b'.' => (Token::Dot, 1),
                _ => {
                    return Err(ExpressionError::Parse {
                        message: format!("unexpected character {:?}", char::from(b)),
                    });
                }
            },
        };
        tokens.push(tok);
        i += advance;
    }
    Ok(tokens)
}

const fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

const fn is_ident_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn read_ident(source: &str, start: usize) -> (String, usize) {
    let bytes = source.as_bytes();
    let mut i = start + 1;
    while i < bytes.len() && is_ident_cont(bytes[i]) {
        i += 1;
    }
    (source[start..i].to_owned(), i)
}

fn read_int(source: &str, start: usize) -> Result<(i64, usize), ExpressionError> {
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    source[start..i]
        .parse::<i64>()
        .map(|n| (n, i))
        .map_err(|_| ExpressionError::Parse {
            message: "invalid integer literal".to_owned(),
        })
}

fn read_string(source: &str, start: usize) -> Result<(String, usize), ExpressionError> {
    let bytes = source.as_bytes();
    let mut i = start + 1;
    let mut out = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Ok((out, i + 1)),
            b'\\' => {
                i += 1;
                let Some(esc) = bytes.get(i).copied() else {
                    return Err(ExpressionError::Parse {
                        message: "unterminated string escape".to_owned(),
                    });
                };
                match esc {
                    b'"' | b'\\' => out.push(char::from(esc)),
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    _ => {
                        return Err(ExpressionError::Parse {
                            message: format!("invalid escape \\{}", char::from(esc)),
                        });
                    }
                }
                i += 1;
            }
            c => {
                out.push(char::from(c));
                i += 1;
            }
        }
    }
    Err(ExpressionError::Parse {
        message: "unterminated string".to_owned(),
    })
}
