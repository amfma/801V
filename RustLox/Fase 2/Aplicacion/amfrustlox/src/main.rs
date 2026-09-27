use std::{env, fmt::format, io::{self, Write, stdin}};

use crate::token::{Scanner, Token};

mod token;
mod parser;
mod error;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        run_rompt();
    }
}

fn run_rompt() {
    let mut buffer: String = String::new();

    loop {
        print!(">>>");
        io::stdout().flush().expect("Console flush error");
        std::io::stdin().read_line(&mut buffer).expect("Expect input to be read");
        buffer.push(';');
        let scanner = Scanner::new(&buffer);
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();

        for token in tokens {
            println!("{:?}", token)
        }
    }
}