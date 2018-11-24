use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Array(Rc<RefCell<Vec<Value>>>),
    Range(f64, f64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "{}", "nil"),
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(a) => write!(f, "Array({:p})", a),
            Value::Range(l, r) => write!(f, "{}..{}", l, r),
        }
    }
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Boolean(b) => !b,

            _ => false,
        }
    }
}
