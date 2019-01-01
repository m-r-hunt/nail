use super::vm::InterpreterError;
use std::cmp::Ordering;
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
    Callable(usize),
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
            Err(InterpreterError::RuntimeError(
                "Tried to hash bad float.".to_string(),
                line,
            ))
        } else {
            let (mantissa, exponent, sign) = value.integer_decode();
            Ok(SanitizedFloat {
                mantissa,
                exponent,
                sign,
            })
        }
    }

    fn to_f64(&self) -> f64 {
        let sign_f = f64::from(self.sign);
        let mantissa_f = self.mantissa as f64;
        let exponent_f = 2.0_f64.powf(f64::from(self.exponent));
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
    Callable(usize),
}

// TL;DR Different enum cases always compare less/equal based on their order in the enum.
// Within a case, some kind of sensible order is used:
// Number - PartialOrd of converted f64 should be guaranteed to work (no NaNs etc)
// Bool - false < true
// String - usual String order
// ReferenceId - a weird one, by Id number order. Kind of like sorting by memory address
// Range - Sort by number order of l value. Arbitrary, again I'm assuming this won't be used much
impl Ord for HashableValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            HashableValue::Nil => match other {
                HashableValue::Nil => Ordering::Equal,
                _ => Ordering::Less,
            },
            HashableValue::Number(sf) => match other {
                HashableValue::Nil => Ordering::Greater,
                HashableValue::Number(sf2) => sf.to_f64().partial_cmp(&sf2.to_f64()).unwrap(),
                _ => Ordering::Less,
            },
            HashableValue::Boolean(b) => match other {
                HashableValue::Nil | HashableValue::Number(_) => Ordering::Greater,
                HashableValue::Boolean(b2) => {
                    if *b == *b2 {
                        Ordering::Equal
                    } else if *b && !*b2 {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                }
                _ => Ordering::Less,
            },
            HashableValue::String(s) => match other {
                HashableValue::String(s2) => s.cmp(s2),
                HashableValue::ReferenceId(_)
                | HashableValue::Range(..)
                | HashableValue::Callable(..) => Ordering::Less,
                _ => Ordering::Greater,
            },
            HashableValue::ReferenceId(id) => match other {
                HashableValue::ReferenceId(id2) => id.cmp(id2),
                HashableValue::Range(..) | HashableValue::Callable(..) => Ordering::Less,
                _ => Ordering::Greater,
            },
            HashableValue::Range(l, _) => match other {
                HashableValue::Range(l2, _) => l.to_f64().partial_cmp(&l2.to_f64()).unwrap(),
                HashableValue::Callable(..) => Ordering::Less,
                _ => Ordering::Greater,
            },
            HashableValue::Callable(c) => match other {
                HashableValue::Callable(c2) => c.cmp(c2),
                _ => Ordering::Greater,
            },
        }
    }
}

impl PartialOrd for HashableValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl HashableValue {
    pub fn try_from(value: &Value, line: usize) -> Result<Self, InterpreterError> {
        match value {
            Value::Nil => Ok(HashableValue::Nil),
            Value::Number(f) => Ok(HashableValue::Number(SanitizedFloat::try_from(*f, line)?)),
            Value::Boolean(b) => Ok(HashableValue::Boolean(*b)),
            Value::String(s) => Ok(HashableValue::String(s.clone())),
            Value::ReferenceId(i) => Ok(HashableValue::ReferenceId(*i)),
            Value::Range(l, r) => Ok(HashableValue::Range(
                SanitizedFloat::try_from(*l, line)?,
                SanitizedFloat::try_from(*r, line)?,
            )),
            Value::MapForContext(..) => Err(InterpreterError::RuntimeError(
                "Tried to hash map for context, this should never happen.".to_string(),
                line,
            )),
            Value::Callable(c) => Ok(HashableValue::Callable(*c)),
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
            HashableValue::Callable(c) => Value::Callable(*c),
        }
    }
}

pub trait ExternalType {
    fn get_arity(&self, name: &str) -> usize;
    fn call(&mut self, name: &str, args: Vec<Value>) -> ValueOrRef;
}

use regex::Regex;
impl ExternalType for Regex {
    fn get_arity(&self, name: &str) -> usize {
        if name == "match" {
            1
        } else {
            panic!("Bad call to regex.")
        }
    }

    fn call(&mut self, name: &str, args: Vec<Value>) -> ValueOrRef {
        if name == "match" {
            if let Value::String(ref s) = args[0] {
                match self.captures(&s) {
                    Some(c) => {
                        return ValueOrRef::Ref(ReferenceType::Array(
                            c.iter()
                                .map(|e| Value::String(e.unwrap().as_str().to_string()))
                                .collect(),
                        ))
                    }
                    None => return ValueOrRef::Value(Value::Nil),
                }
            } else {
                panic!("Bad call to regex match");
            }
        } else {
            panic!("Bad call to regex.")
        }
    }
}

pub enum ReferenceType {
    Array(Vec<Value>),
    Map(HashMap<HashableValue, Value>),
    External(Box<dyn ExternalType>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::ReferenceId(i) => write!(f, "RefId({})", i),
            Value::Range(l, r) => write!(f, "{}..{}", l, r),
            Value::MapForContext(..) => panic!("Attempted to display map for context."),
            Value::Callable(c) => write!(f, "Callable({})", c),
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

pub enum ValueOrRef {
    Value(Value),
    Ref(ReferenceType),
}
