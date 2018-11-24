#[derive(Debug)]
pub enum NotloxError {
    ScannerError(String),
    ParserError(String, usize),
    CompilerError(String),
}

impl std::fmt::Display for NotloxError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::NotloxError::*;
        write!(
            f,
            "{}",
            match self {
                ScannerError(e) => format!("Scanner error: {}", e),
                ParserError(e, n) => format!("Parser error: line({}): {}", n, e),
                CompilerError(e) => format!("Compiler error: {}", e),
            }
        )
    }
}

pub type Result<T> = std::result::Result<T, NotloxError>;
