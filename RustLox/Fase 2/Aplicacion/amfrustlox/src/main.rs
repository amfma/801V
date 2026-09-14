use std::{env, io::{self, Write, stdin}};

mod token;

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
    }
}