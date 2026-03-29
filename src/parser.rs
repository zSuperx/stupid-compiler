use core::panic;
use std::collections::VecDeque;

use crate::types::*;

pub struct Parser<'src> {
    src: &'src [Kind<'src>],
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src [Kind<'src>]) -> Self {
        Self { src }
    }

    fn is_next(&self, kind: Kind<'src>) -> bool {
        self.peek().clone() == kind
    }

    fn expect(&mut self, kind: Kind<'src>) {
        let next = self.consume();
        if next != kind {
            panic!("Expected {kind}, found {next}");
        }
    }

    fn expect_ident(&mut self) -> &'src str {
        let next = self.consume();
        match next {
            Kind::Ident(s) => s,
            _ => panic!("Expected identifier, found {next}"),
        }
    }

    fn expect_strlit(&mut self) -> &'src [u8] {
        let next = self.consume();
        match next {
            Kind::Str(s) => s,
            _ => panic!("Expected string literal, found {next}"),
        }
    }

    fn expect_intlit(&mut self) -> u64 {
        let next = self.consume();
        match next {
            Kind::Int(s) => s,
            _ => panic!("Expected int literal, found {next}"),
        }
    }

    fn consume(&mut self) -> Kind<'src> {
        let first = self.src.first().unwrap_or(&Kind::EOF);
        if !self.src.is_empty() {
            self.src = &self.src[1..];
        }
        first.clone()
    }

    fn peek(&self) -> Kind<'src> {
        self.src.first().unwrap_or(&Kind::EOF).clone()
    }

    fn parse_program(&mut self) -> Vec<Object<'src>> {
        let mut objs = vec![];
        while !self.is_next(Kind::EOF) {
            match self.peek() {
                Kind::Fn => objs.push(self.parse_fn()),
                x => panic!("Expected function definition, found {x}"),
            }
        }
        objs
    }

    // This parses the following phrase:
    // VARNAME ":" TYPE
    fn parse_variable(&mut self) -> Variable<'src> {
        let Kind::Ident(name) = self.consume() else {
            panic!("Expected identifier");
        };
        self.expect(Kind::Colon);
        let ty = self.parse_type();
        Variable {
            name: name.to_string(),
            ty,
        }
    }

    fn parse_fn(&mut self) -> Object<'src> {
        self.expect(Kind::Fn);
        let name = self.expect_ident();
        self.expect(Kind::LParen);

        let mut args = vec![];
        if !self.is_next(Kind::RParen) {
            args.push(self.parse_variable());
        }

        loop {
            let cur = self.peek();
            if matches!(cur, Kind::RParen) {
                self.consume();
                break;
            }

            self.expect(Kind::Comma);

            args.push(self.parse_variable());
        }

        let returns = if self.is_next(Kind::Arrow) {
            self.expect(Kind::Arrow);
            Some(self.parse_type())
        } else {
            None
        };

        let body = self.parse_block();

        Object::FnDef {
            name,
            returns,
            args,
            body,
        }
    }

    fn parse_type(&mut self) -> Type<'src> {
        let mut ty = match self.peek() {
            Kind::Star => {
                self.consume();
                return Type::Pointer(Box::new(self.parse_type()));
            }
            Kind::Ident(name) => Type::Base(name),
            x => panic!("Expected type, found {x}"),
        };
        self.consume();
        ty
    }

    fn parse_prefix(&mut self) -> Expr<'src> {
        match self.consume() {
            Kind::Ident(s) => Expr::Ident(s),
            Kind::Int(x) => Expr::Int(x),
            Kind::Str(s) => Expr::Str(s),
            op @ (Kind::Minus | Kind::Bang | Kind::And | Kind::Star) => Expr::Unary {
                op: match op {
                    Kind::Minus => UnOp::Negate,
                    Kind::Bang => UnOp::Not,
                    Kind::And => UnOp::AddrOf,
                    Kind::Star => UnOp::Deref,
                    _ => unreachable!(),
                },
                rhs: Box::new(self.parse_expr(prefix_power(op).expect("Invalid operator"))),
            },
            Kind::LParen => {
                let inner = self.parse_expr(0.0);
                self.expect(Kind::RParen);
                inner
            }
            x => panic!("Expected start of expression, found \"{x}\""),
        }
    }

    fn parse_infix(&mut self, lhs: Expr<'src>, op: Kind<'src>, op_power: f32) -> Expr<'src> {
        match op {
            Kind::LParen => {
                let mut args = vec![];
                while !self.is_next(Kind::RParen) {
                    if !args.is_empty() {
                        self.expect(Kind::Comma);
                    }
                    args.push(self.parse_expr(0.0));
                }
                self.expect(Kind::RParen);
                Expr::Call {
                    callee: Box::new(lhs),
                    args,
                }
            }
            op @ (Kind::Eq
            | Kind::Plus
            | Kind::Minus
            | Kind::Star
            | Kind::Slash
            | Kind::AndAnd
            | Kind::OrOr
            | Kind::EqEq
            | Kind::BangEq
            | Kind::Lt
            | Kind::LtEq
            | Kind::Gt
            | Kind::GtEq) => {
                let rhs = self.parse_expr(op_power);
                let binop = match op {
                    Kind::Eq => BinOp::Assign,
                    Kind::Plus => BinOp::Add,
                    Kind::Minus => BinOp::Sub,
                    Kind::Star => BinOp::Mul,
                    Kind::Slash => BinOp::Div,
                    Kind::AndAnd => BinOp::LogAnd,
                    Kind::OrOr => BinOp::LogOr,
                    Kind::EqEq => BinOp::Eq,
                    Kind::BangEq => BinOp::Ne,
                    Kind::Lt => BinOp::Lt,
                    Kind::LtEq => BinOp::Le,
                    Kind::Gt => BinOp::Gt,
                    Kind::GtEq => BinOp::Ge,
                    _ => unreachable!(),
                };
                Expr::Bin {
                    op: binop,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            Kind::LBrack => {
                let index = self.parse_expr(op_power);
                self.expect(Kind::RBrack);
                Expr::Bin {
                    op: BinOp::Index,
                    lhs: Box::new(lhs),
                    rhs: Box::new(index),
                }
            }
            Kind::Dot => {
                let field = self.parse_expr(op_power);
                if !matches!(field, Expr::Ident(_)) {
                    panic!("Fields can only be accessed with an identifier, not a {field:?}");
                }
                Expr::Bin {
                    op: BinOp::FieldAccess,
                    lhs: Box::new(lhs),
                    rhs: Box::new(field),
                }
            }
            x => panic!("Expected infix operator, found {x}"),
        }
    }

    fn parse_expr(&mut self, min_power: f32) -> Expr<'src> {
        let mut lhs = self.parse_prefix();

        loop {
            let op = self.peek();

            let Some(op_power) = infix_power(op) else {
                break;
            };

            if op_power.0 < min_power {
                break;
            }

            self.consume();

            lhs = self.parse_infix(lhs, op, op_power.1);
        }

        lhs
    }

    fn parse_block(&mut self) -> Stmt<'src> {
        self.expect(Kind::LCurly);
        let mut stmts = vec![];
        while !self.is_next(Kind::RCurly) {
            let stmt = self.parse_stmt();
            if matches!(stmt, Stmt::Block(ref inner) if inner.is_empty()) {
                // Skip nested empty {} blocks, they're useless.
                continue;
            }
            stmts.push(stmt);
        }
        self.expect(Kind::RCurly);
        Stmt::Block(stmts)
    }

    fn parse_stmt(&mut self) -> Stmt<'src> {
        let stmt = match self.peek() {
            Kind::Let => {
                self.expect(Kind::Let);
                let lhs = self.parse_variable();

                self.expect(Kind::Eq);

                let rhs = self.parse_expr(0.0);
                self.expect(Kind::Semi);
                Stmt::Let { lhs, rhs }
            }
            Kind::While => {
                self.expect(Kind::While);
                let cond = self.parse_expr(0.0);
                let body = self.parse_stmt();
                Stmt::While {
                    cond,
                    body: Box::new(body),
                }
            }
            Kind::Continue => {
                self.expect(Kind::Continue);
                Stmt::Continue
            }
            Kind::Break => {
                self.expect(Kind::Break);
                Stmt::Break
            }
            Kind::If => {
                self.expect(Kind::If);
                let cond = self.parse_expr(0.0);
                let then_ = self.parse_block();

                let else_ = if self.is_next(Kind::Else) {
                    self.expect(Kind::Else);
                    if self.is_next(Kind::If) {
                        // If is a special case since it's allowed after an else
                        self.parse_stmt()
                    } else {
                        // Otherwise, else must be followed by a block
                        self.parse_block()
                    }
                } else {
                    Stmt::Block(vec![])
                };
                Stmt::If {
                    cond,
                    then_: Box::new(then_),
                    else_: Box::new(else_),
                }
            }
            Kind::Return => {
                self.expect(Kind::Return);
                let retval = if !self.is_next(Kind::Semi) {
                    Some(self.parse_expr(0.0))
                } else {
                    None
                };
                self.expect(Kind::Semi);

                Stmt::Return(retval)
            }
            _ => {
                let expr = self.parse_expr(0.0);
                self.expect(Kind::Semi);
                Stmt::Expr(expr)
            }
        };
        stmt
    }
}

fn prefix_power(kind: Kind) -> Option<f32> {
    let power = match kind {
        Kind::Bang | Kind::Minus | Kind::Star | Kind::And => 9.0,
        _ => return None, // Not an prefix operator
    };
    Some(power)
}

fn infix_power(kind: Kind) -> Option<(f32, f32)> {
    let power = match kind {
        // Assignment: Right-Associative
        // We use a lower Left power so it "gives up" easily,
        // but a higher Right power to "grab" everything to the right.
        Kind::Eq => (2.1, 2.0),

        Kind::AndAnd | Kind::OrOr => (4.1, 4.0),

        Kind::EqEq | Kind::BangEq => (5.0, 5.1),
        Kind::Lt | Kind::Gt => (6.0, 6.1),
        Kind::LtEq | Kind::GtEq => (6.0, 6.1),

        Kind::Plus | Kind::Minus => (7.0, 7.1),

        Kind::Star | Kind::Slash => (8.0, 8.1),

        // Postfix: Highest priority
        // (Call, Indexing, Member Access)
        Kind::LParen | Kind::LBrack | Kind::Dot => (10.0, 10.1),

        _ => return None, // Not an infix operator
    };
    Some(power)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tokenize::Lexer, types::*};
    #[test]
    fn test_parse_type() {
        let src = b"u32 *billy";
        let tokens: Vec<Kind> = Lexer::new(src).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let x = parser.parse_type();
        dbg!(x);
    }

    #[test]
    fn test_parse_expr() {
        let input = b"x = y = hi";
        println!("input = {}", str::from_utf8(input).unwrap());
        let tokens: Vec<Kind> = Lexer::new(input).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let output = parser.parse_expr(0.0);
        dbg!(output);
    }

    #[test]
    fn test_parse_stmt() {
        let input = b"if true";
        println!("input = {}", str::from_utf8(input).unwrap());
        let tokens: Vec<Kind> = Lexer::new(input).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let output = parser.parse_expr(0.0);
        dbg!(output);
    }

    #[test]
    fn test_parse_fn() {
        let src = br#"
fn bob(arg1: *u32, arg2: u8) -> u32 {
    42.foo();
}
"#;
        let tokens: Vec<Kind> = Lexer::new(src).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let parsed = parser.parse_fn();
        println!("Source code: {}", str::from_utf8(src).unwrap());
        dbg!(parsed);
    }

    #[test]
    fn test_parse_program() {
        let src = std::fs::read("tests/test.stupid").unwrap();
        let tokens: Vec<Kind> = Lexer::new(&src).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let parsed = parser.parse_program();
        println!("Source code: {}", str::from_utf8(&src).unwrap());
        dbg!(parsed);
    }
}
