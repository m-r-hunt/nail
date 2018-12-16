use notlox;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        notlox::repl();
    } else if args.len() == 2 {
        notlox::run_file(&args[1]);
    } else {
        println!("Usage: clox [path]");
    }
}
