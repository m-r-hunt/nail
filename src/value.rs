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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SanitizedFloat {
    pub mantissa: u64,
    pub exponent: i16,
    pub sign: i8,
}

impl SanitizedFloat {
    fn try_from(value: f64) -> Result<Self, InterpreterError> {
        use num::Float;
        if !value.is_finite() {
            return Err(InterpreterError::RuntimeError(
                "Tried to hash bad float.".to_string(),
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
    pub fn try_from(value: Value) -> Result<Self, InterpreterError> {
        match value {
            Value::Nil => Ok(HashableValue::Nil),
            Value::Number(f) => Ok(HashableValue::Number(SanitizedFloat::try_from(f)?)),
            Value::Boolean(b) => Ok(HashableValue::Boolean(b)),
            Value::String(s) => Ok(HashableValue::String(s)),
            Value::ReferenceId(i) => Ok(HashableValue::ReferenceId(i)),
            Value::Range(l, r) => Ok(HashableValue::Range(
                SanitizedFloat::try_from(l)?,
                SanitizedFloat::try_from(r)?,
            )),
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
