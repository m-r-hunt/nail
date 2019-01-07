use nail;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        nail::repl();
    } else if args.len() == 2 {
        nail::run_file(&args[1]);
    } else {
        println!("Usage: clox [path]");
    }
}
