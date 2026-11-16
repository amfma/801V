use std::{cell::RefCell, env, io, process, rc::Rc};

use amfrustlox::{interpreter::Interpreter, runner};

fn main() {
    // Lox recursion maps onto Rust recursion: run on a thread with a big stack.
    let code = runner::with_big_stack(real_main);
    process::exit(code);
}

fn real_main() -> i32 {
    let args: Vec<String> = env::args().collect();
    let mut interpreter = Interpreter::new(Rc::new(RefCell::new(io::stdout())));

    match args.len() {
        1 => {
            let stdin = io::stdin();
            if let Err(error) =
                runner::repl(&mut stdin.lock(), &mut io::stdout(), &mut interpreter)
            {
                eprintln!("I/O error: {error}");
                return 74;
            }
            0
        }
        2 => runner::run_file(&mut interpreter, &args[1]),
        _ => {
            eprintln!("Usage: amfrustlox [script]");
            64
        }
    }
}
