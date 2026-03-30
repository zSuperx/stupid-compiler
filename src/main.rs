#![allow(unused)]

use crate::lexer::Lexer;

mod lexer;
mod parser;
mod types;
mod resolver;
mod codegen;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: stupid <FILE>");
        return;
    }

    let file = &args[1];
    let content = std::fs::read(file).unwrap();

    let mut lexer = Lexer::new(&content);

    for token in lexer {
        println!("{}", token.kind);
    }
}
