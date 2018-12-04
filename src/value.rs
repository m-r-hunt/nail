use super::vm::InterpreterError;
use std::collections::hash_map::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    ReferenceId(usize),
    Range(f64, f64),
    MapForContext(Vec<HashableValue>, f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SanitizedFloat {
    pub mantissa: u64,
    pub exponent: i16,
    pub sign: i8,
}

impl SanitizedFloat {
    fn try_from(value: f64, line: usize) -> Result<Self, InterpreterError> {
        use num::Float;
        if !value.is_finite() {
            return Err(InterpreterError::RuntimeError(
                "Tried to hash bad float.".to_string(),
                line,
            ));
        } else {
            let (mantissa, exponent, sign) = value.integer_decode();
            return Ok(SanitizedFloat {
                mantissa,
                exponent,
                sign,
            });
        }
    }

    fn to_f64(&self) -> f64 {
        let sign_f = self.sign as f64;
        let mantissa_f = self.mantissa as f64;
        let exponent_f = 2.0_f64.powf(self.exponent as f64);
        sign_f * mantissa_f * exponent_f
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HashableValue {
    Nil,
    Number(SanitizedFloat),
    Boolean(bool),
    String(String),
    ReferenceId(usize),
    Range(SanitizedFloat, SanitizedFloat),
}

impl HashableValue {
    pub fn try_from(value: Value, line: usize) -> Result<Self, InterpreterError> {
        match value {
            Value::Nil => Ok(HashableValue::Nil),
            Value::Number(f) => Ok(HashableValue::Number(SanitizedFloat::try_from(f, line)?)),
            Value::Boolean(b) => Ok(HashableValue::Boolean(b)),
            Value::String(s) => Ok(HashableValue::String(s)),
            Value::ReferenceId(i) => Ok(HashableValue::ReferenceId(i)),
            Value::Range(l, r) => Ok(HashableValue::Range(
                SanitizedFloat::try_from(l, line)?,
                SanitizedFloat::try_from(r, line)?,
            )),
            Value::MapForContext(..) => Err(InterpreterError::RuntimeError(
                "Tried to hash map for context, this should never happen.".to_string(),
                line,
            )),
        }
    }
}

impl Value {
    pub fn from(value: &HashableValue) -> Self {
        match value {
            HashableValue::Nil => Value::Nil,
            HashableValue::Number(f) => Value::Number(f.to_f64()),
            HashableValue::Boolean(b) => Value::Boolean(*b),
            HashableValue::String(s) => Value::String(s.clone()),
            HashableValue::ReferenceId(i) => Value::ReferenceId(*i),
            HashableValue::Range(l, r) => Value::Range(l.to_f64(), r.to_f64()),
        }
    }
}

pub enum ReferenceType {
    Nil,
    Array(Vec<Value>),
    Map(HashMap<HashableValue, Value>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "{}", "nil"),
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::ReferenceId(i) => write!(f, "RefId({})", i),
            Value::Range(l, r) => write!(f, "{}..{}", l, r),
            Value::MapForContext(..) => panic!("Attempted to display map for context."),
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
    pub fn is_truey(&self) -> bool {
        !self.is_falsey()
    }
}
