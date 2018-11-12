#[derive(Copy, Clone)]
pub enum Value {
    Nil,
    Number(f64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "{}", "nil"),
            Value::Number(n) => write!(f, "{}", n),
        }
    }
}
