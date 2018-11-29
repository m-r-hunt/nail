extern crate num;

pub mod chunk;
mod compiler;
pub mod debug;
mod errors;
mod parser;
pub mod scanner;
mod value;
pub mod vm;

use std::io::Write;
use std::time::Instant;

pub fn repl() {
    let mut vm = vm::VM::new();
    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();

        let mut line = String::new();

        let result = std::io::stdin().read_line(&mut line);
        match result {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
                break;
            }
        }

        let result = vm.interpret(&format!("fn main() {{{}}}", line));
        match result {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
            }
        }
    }
}

pub fn run_file(filename: &str) {
    let start = Instant::now();
    let result = std::fs::read_to_string(filename);
    let code = result.expect(&format!("Unable to read file {}", filename));
    let read_file_done = Instant::now();

    let mut vm = vm::VM::new();
    let result = vm.interpret(&code);
    match result {
        Ok(_) => {}
        Err(e) => {
            println!("{}", e);
            return;
        }
    }
    let finished = Instant::now();
    println!("Done. File read: {}s {}ms, Interpreted: {}s {}ms.", read_file_done.duration_since(start).as_secs(), read_file_done.duration_since(start).subsec_millis(), finished.duration_since(read_file_done).as_secs(), finished.duration_since(read_file_done).subsec_millis());
}
