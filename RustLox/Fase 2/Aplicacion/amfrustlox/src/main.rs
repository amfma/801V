use std::{cell::{Ref, RefCell}, env, fmt::format, io::{self, Write, stdin}, rc::Rc};

use crate::{resolver::Resolver, token::{Scanner, Token}};
use crate::parser::{Expr, Parser};
use crate::interpreter::{Interpreter};

mod token;
mod parser;
mod error;
mod interpreter;
mod enviroment;
mod callable;
mod resolver;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        run_rompt();
    }
}

fn run_rompt() {
    let mut interpreter: Interpreter = Interpreter::new(Rc::new(RefCell::new(io::stdout())));

    loop {
        let mut buffer: String = String::new();
        print!(">>>");
        io::stdout().flush().expect("Console flush error");
        std::io::stdin().read_line(&mut buffer).expect("Expect input to be read");
        //buffer.push();
        let scanner = Scanner::new(&buffer);
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();
        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(statements) => {
                let resolved = Resolver::new(&mut interpreter).resolve(&statements);
                match resolved {
                    Ok(statements) => interpreter.interpret(&statements),
                    Err(_) => println!("Resolver error")
                }
            }
            Err(_) => println!("Parser error"),
        }
    }
}