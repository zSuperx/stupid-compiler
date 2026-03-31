#![allow(unused)]

use crate::{emitter::Emitter, lexer::Lexer, parser::Parser, resolver::Resolver};

mod lexer;
mod parser;
mod types;
mod resolver;
mod emitter;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: stupid <FILE>");
        return;
    }

    let file = &args[1];
    let content = std::fs::read(file).unwrap();

    let lexed = Lexer::new(&content).map(|t| t.kind).collect::<Vec<_>>();
    let parsed = Parser::new(&lexed).parse_program();
    let resolved = Resolver::new().resolve_program(&parsed);
    let emitted = Emitter::new().emit_program(&resolved);
    println!("{emitted}");
}
