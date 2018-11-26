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
    let result = std::fs::read_to_string(filename);
    let code = result.expect(&format!("Unable to read file {}", filename));

    let mut vm = vm::VM::new();
    let result = vm.interpret(&code);
    match result {
        Ok(_) => {}
        Err(e) => {
            println!("{}", e);
            return;
        }
    }
}
