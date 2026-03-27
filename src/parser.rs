use std::collections::{HashMap, LinkedList};

use crate::tokenize::{Token, TokenKind};

struct Member {}

enum Type {
    U8,
    U32,
    Struct(Vec<Member>),
}

pub struct Parser {
    types: HashMap<String, Type>,
}

impl Parser {
    pub fn new() -> Self {
        let mut types = HashMap::new();
        types.insert("u8".into(), Type::U8).unwrap();
        types.insert("u32".into(), Type::U8).unwrap();

        Self { types }
    }
    // u32 x = 5;
    // u8 y = 6;
    // myType t = {}; (myType must exist in Types via parse_typedef)
    fn parse_declspec(&self, tok: &[Token]) -> Option<&Type> {
        match &tok.get(0)?.kind {
            TokenKind::Ident(i) => self.types.get(i),
            TokenKind::Keyword(k) => match k.as_str() {
                builtin @ ("u8" | "u32") => self.types.get(builtin),
                x => panic!("Unknown type \"{x}\""),
            },
            x => panic!("Expected type, got {x}"),
        }
    }

    pub fn parse_program(tok: &[Token]) {}

    fn parse_variable_declarator(tok: &[Token]) {

    }

    fn parse_function(tok: &[Token]) {}

    fn parse_typedef(tok: &[Token]) {}
}
