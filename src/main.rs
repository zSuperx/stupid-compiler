use crate::{emitter::Emitter, lexer::Lexer, parser::Parser, resolver::Resolver, types::TKind};

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

    let lexed = Lexer::new(&content).filter(|t| t.kind != TKind::Whitespace).collect::<Vec<_>>();
    // println!("{lexed:#?}");
    let parsed = Parser::new(&lexed).parse_program();
    // println!("{parsed:#?}");
    let resolved = Resolver::new().resolve_program(&parsed);
    // println!("{resolved:#?}");
    let emitted = Emitter::new().emit_program(&resolved).join("");
    println!("{emitted}");
}
