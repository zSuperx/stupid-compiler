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

    let sym = ctx.to_symbol("u8");
    let type_id = ctx.declare_type(sym, 1);
    ctx.types[type_id].is_primitive = true;
    ctx.types[type_id].is_integral = true;

    let sym = ctx.to_symbol("i8");
    let type_id = ctx.declare_type(sym, 1);
    ctx.types[type_id].is_primitive = true;
    ctx.types[type_id].is_integral = true;
    ctx.types[type_id].is_signed = true;

    let sym = ctx.to_symbol("u16");
    let type_id = ctx.declare_type(sym, 2);
    ctx.types[type_id].is_primitive = true;
    ctx.types[type_id].is_integral = true;

    let sym = ctx.to_symbol("i16");
    let type_id = ctx.declare_type(sym, 2);
    ctx.types[type_id].is_primitive = true;
    ctx.types[type_id].is_integral = true;
    ctx.types[type_id].is_signed = true;

    let sym = ctx.to_symbol("u32");
    let type_id = ctx.declare_type(sym, 4);
    ctx.types[type_id].is_primitive = true;
    ctx.types[type_id].is_integral = true;

    let sym = ctx.to_symbol("i32");
    let type_id = ctx.declare_type(sym, 4);
    ctx.types[type_id].is_primitive = true;
    ctx.types[type_id].is_integral = true;
    ctx.types[type_id].is_signed = true;

    let sym = ctx.to_symbol("u64");
    let type_id = ctx.declare_type(sym, 8);
    ctx.types[type_id].is_primitive = true;
    ctx.types[type_id].is_integral = true;

    let sym = ctx.to_symbol("i64");
    let type_id = ctx.declare_type(sym, 8);
    ctx.types[type_id].is_primitive = true;
    ctx.types[type_id].is_integral = true;
    ctx.types[type_id].is_signed = true;

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
    println!("{ctx:?}");
}
