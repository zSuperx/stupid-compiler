#![allow(unused)]
// use crate::{emitter::Emitter, parser::Parser, resolver::Resolver};
use crate::{lexer::Lexer, parser::Parser, types::Context};

mod types;
mod lexer;
mod parser;
mod resolver;
// mod emitter;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: stupid <FILE>");
        return;
    }

    let file = &args[1];
    let content = std::fs::read(file).unwrap();

    let mut ctx = Context::new(content.clone());
    let mut lexer = Lexer::new(&content);

    let mut tokens = vec![];
    while let Some(tok) = lexer.next_token(&mut ctx) {
        tokens.push(tok);
    }
    // println!("{lexed:#?}");
    let parsed = Parser::new(&tokens).parse_program(&mut ctx);
    println!("{parsed:#?}");
    // let resolved = Resolver::new().resolve_program(&parsed);
    // // println!("{resolved:#?}");
    // let emitted = Emitter::new().emit_program(&resolved);
    // for instruction in emitted {
    //     println!("{instruction}")
    // }
}
