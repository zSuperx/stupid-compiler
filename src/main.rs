#![allow(unused)]
use crate::{parser::parse_program, tokenize::Lexer};

mod parser;
mod tokenize;
mod types;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: stupid <FILE>");
        return;
    }

    let file = &args[1];

    let toks = Lexer::new(file).tokenize();
    for tok in toks.iter() {
        println!("{:?}", tok.kind);
    }

    parse_program(tok);
}
