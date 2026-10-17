use std::{env, fmt::format, io::{self, Write, stdin}};

use crate::token::{Scanner, Token};
use crate::parser::{Expr, Parser};
use crate::interpreter::{Interpreter};

mod token;
mod parser;
mod error;
mod interpreter;
mod enviroment;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        run_rompt();
    }
}

fn run_rompt() {
    let mut interpreter: Interpreter = Interpreter::new();

    loop {
        let mut buffer: String = String::new();
        print!(">>>");
        io::stdout().flush().expect("Console flush error");
        std::io::stdin().read_line(&mut buffer).expect("Expect input to be read");
        //buffer.push();
        let scanner = Scanner::new(&buffer);
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        match ast {
            Ok(statements) => interpreter.interpret(&statements),
            Err(_) => println!("Error"),
        }
    }
}