use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct ShittyMap {
    keys: Vec<Value>,
    values: Vec<Value>,
}

impl ShittyMap {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: Value, value: Value) {
        for (i, k) in self.keys.iter().enumerate() {
            if *k == key {
                self.values[i] = value;
                return;
            }
        }
        self.keys.push(key);
        self.values.push(value);
    }

    pub fn lookup(&self, key: Value) -> Value {
        for (i, k) in self.keys.iter().enumerate() {
            if *k == key {
                return self.values[i].clone();
            }
        }
        return Value::Nil;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Array(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<ShittyMap>>),
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
            Value::Map(m) => write!(f, "Map{:p})", m),
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
