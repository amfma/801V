use std::{cell::RefCell, env, io, process, rc::Rc};

use amfrustlox::{interpreter::Interpreter, runner};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut interpreter = Interpreter::new(Rc::new(RefCell::new(io::stdout())));

    match args.len() {
        1 => {
            let stdin = io::stdin();
            if let Err(error) =
                runner::repl(&mut stdin.lock(), &mut io::stdout(), &mut interpreter)
            {
                eprintln!("I/O error: {error}");
                process::exit(74);
            }
        }
        2 => process::exit(runner::run_file(&mut interpreter, &args[1])),
        _ => {
            eprintln!("Usage: amfrustlox [script]");
            process::exit(64);
        }
    }
}
