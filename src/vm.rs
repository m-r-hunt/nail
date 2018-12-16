use super::{chunk, compiler, debug, errors::NotloxError, value::*};
use std::collections::HashMap;
use std::time::Instant;

const STACK_SIZE: usize = 256;

#[derive(Copy, Clone, Debug)]
struct CallFrame {
    return_address: usize,
    locals_base: usize,
}

struct ValueStack {
    stack: Vec<Value>,
    top: usize,
}

impl ValueStack {
    fn new() -> Self {
        let mut array = Vec::new();
        array.resize(STACK_SIZE, Value::Nil);
        ValueStack {
            stack: array,
            top: 0,
        }
    }

    fn push(&mut self, value: Value) {
        self.stack[self.top] = value;
        self.top += 1
    }

    fn pop(&mut self, line: usize) -> Result<Value, InterpreterError> {
        if self.top >= 1 {
            self.top -= 1;
            return Ok(self.stack[self.top].clone());
        } else {
            return runtime_error("Not enough values on the stack", line);
        }
    }

    fn pop_multi(&mut self, n: usize, line: usize) -> Result<Value, InterpreterError> {
        if self.top >= n {
            self.top -= n;
            return Ok(Value::Nil);
        } else {
            return runtime_error("Not enough values on the stack", line);
        }
    }

    fn peek(&mut self) -> Value {
        if self.top >= 1 {
            self.stack[self.top - 1].clone()
        } else {
            Value::Nil
        }
    }
}

pub struct VM {
    chunk: chunk::Chunk,
    ip: usize,
    stack: ValueStack,
    return_stack: [CallFrame; STACK_SIZE],
    return_stack_top: usize,
    locals: Vec<Value>,
    locals_base: usize,
    locals_top: usize,
    heap: Vec<ReferenceType>,
}

#[derive(Debug)]
pub enum InterpreterError {
    CompileError(NotloxError),
    RuntimeError(String, usize),
}

impl From<NotloxError> for InterpreterError {
    fn from(c: NotloxError) -> Self {
        InterpreterError::CompileError(c)
    }
}

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::CompileError(c) => c.fmt(f),
            InterpreterError::RuntimeError(s, line) => {
                write!(f, "Runtime Error, line {}: {}", line, s)
            }
        }
    }
}

impl std::error::Error for InterpreterError {}

fn runtime_error(message: &str, line: usize) -> Result<Value, InterpreterError> {
    Err(InterpreterError::RuntimeError(message.to_string(), line))
}

macro_rules! binary_op {
    ( $self:expr, $op:tt, $type: ident, $ret:ident, $line:expr ) => {
        {
            let a;
            if let Value::$type(aval) = $self.stack.pop($line)? {
                a = aval;
            } else {
                return runtime_error("Bad argument to binary operator, not a number.", $line);
            }
            let b;
            if let Value::$type(bval) = $self.stack.pop($line)? {
                b = bval;
            } else {
                return runtime_error("Bad argument to binary operator, not a number.", $line);
            }
            $self.stack.push(Value::$ret(a $op b))
        }
    }
}

impl VM {
    pub fn new() -> VM {
        let mut array = Vec::new();
        array.resize(STACK_SIZE, Value::Nil);

        VM {
            chunk: chunk::Chunk::new(),
            ip: 0,
            stack: ValueStack::new(),
            return_stack: [CallFrame {
                return_address: 0,
                locals_base: 0,
            }; STACK_SIZE],
            return_stack_top: 0,
            locals: array,
            locals_base: 0,
            locals_top: 0,
            heap: Vec::new(),
        }
    }

    pub fn interpret(&mut self, source: &str) -> Result<Value, InterpreterError> {
        let start = Instant::now();
        let chunk = compiler::compile(source)?;
        let compiled = Instant::now();
        self.chunk = chunk;
        self.ip = self.chunk.lookup_function("main");
        let result = self.run();
        let finished = Instant::now();
        println!(
            "VM Done. Compiled: {}s {}ms, Run: {}s {}ms.",
            compiled.duration_since(start).as_secs(),
            compiled.duration_since(start).subsec_millis(),
            finished.duration_since(compiled).as_secs(),
            finished.duration_since(compiled).subsec_millis()
        );
        result
    }

    pub fn run(&mut self) -> Result<Value, InterpreterError> {
        loop {
            if cfg!(feature = "debugTraceExecution") {
                print!("          ");
                for slot in 0..self.stack.top {
                    print!("[ {:?} ]", self.stack.stack[slot])
                }
                println!();
                debug::disassemble_instruction(&self.chunk, self.ip);
                //let mut buf = [0; 10];
                //use std::io::Read;
                //std::io::stdin().read(&mut buf).unwrap();
            }
            let current_line = self.chunk.lines[self.ip];
            let instruction = self.read_byte();
            match chunk::OpCode::try_from(instruction) {
                Some(chunk::OpCode::Return) => {
                    if self.return_stack_top > 0 {
                        let call_frame = self.return_stack[self.return_stack_top - 1];
                        self.return_stack_top -= 1;
                        self.locals_top = self.locals_base;
                        self.locals_base = call_frame.locals_base;
                        self.ip = call_frame.return_address;
                    } else {
                        return Ok(self.stack.pop(current_line)?);
                    }
                }

                Some(chunk::OpCode::Constant) => {
                    let constant = self.read_constant();
                    self.stack.push(constant);
                }

                Some(chunk::OpCode::Negate) => {
                    if let Value::Number(value) = self.stack.pop(current_line)? {
                        self.stack.push(Value::Number(-value));
                    } else {
                        return runtime_error("Bad argument to negate, not a number.", current_line);
                    }
                }

                Some(chunk::OpCode::Add) => {
                    let top = self.stack.peek();
                    if let Value::Number(_) = top {
                        binary_op!(self, +, Number, Number, current_line)
                    } else if let Value::String(_) = top {
                        let aa = self.stack.pop(current_line)?;
                        let b = self.stack.pop(current_line)?;
                        if let Value::String(b) = b {
                            let a;
                            if let Value::String(aval) = aa {
                                a = aval;
                            } else {
                                panic!("This should be unreachable as we know aa is a string.");
                            }
                            self.stack.push(Value::String(a + &b))
                        } else if let Value::Number(n) = b {
                            let a;
                            if let Value::String(aval) = aa {
                                a = aval;
                            } else {
                                panic!("This should be unreachable as we know aa is a string.");
                            }
                            let mut s = a.clone();
                            s.push(n as u8 as char);
                            self.stack.push(Value::String(s));
                        } else {
                            return runtime_error("Bad argument to binary operator, string must have string or char on RHS.", current_line);
                        }
                    } else {
                        return runtime_error("Bad or mismatched arguments to +", current_line);
                    }
                }
                Some(chunk::OpCode::Subtract) => binary_op!(self, -, Number, Number, current_line),
                Some(chunk::OpCode::Multiply) => binary_op!(self, *, Number, Number, current_line),
                Some(chunk::OpCode::Divide) => binary_op!(self, /, Number, Number, current_line),
                Some(chunk::OpCode::Remainder) => binary_op!(self, %, Number, Number, current_line),

                Some(chunk::OpCode::Print) => println!("{}", self.stack.pop(current_line)?),

                Some(chunk::OpCode::AssignLocal) => {
                    let number = self.read_byte() as usize + self.locals_base;
                    if number >= self.locals_top {
                        return runtime_error("Local store out of range", current_line);
                    }
                    self.locals[number as usize] = self.stack.pop(current_line)?;
                }
                Some(chunk::OpCode::LoadLocal) => {
                    let number = self.read_byte() as usize + self.locals_base;
                    if number >= self.locals_top {
                        return runtime_error("Local load out of range", current_line);
                    }
                    let value = self.locals[number as usize].clone();
                    self.stack.push(value);
                }

                Some(chunk::OpCode::PushNil) => {
                    self.stack.push(Value::Nil);
                }
                Some(chunk::OpCode::Pop) => {
                    self.stack.pop(current_line)?;
                }

                Some(chunk::OpCode::FunctionEntry) => {
                    let localsn = self.read_byte() as usize;
                    self.locals_top = self.locals_base + localsn;
                }
                Some(chunk::OpCode::Call) => {
                    let fn_number = self.read_byte();
                    self.return_stack[self.return_stack_top] = CallFrame {
                        return_address: self.ip,
                        locals_base: self.locals_base,
                    };
                    self.return_stack_top += 1;
                    self.ip = self.chunk.function_locations[fn_number as usize];
                    self.locals_base = self.locals_top;
                }

                Some(chunk::OpCode::JumpIfFalse) => {
                    let target = self.read_signed_16();
                    let value = self.stack.pop(current_line)?;
                    if value.is_falsey() {
                        self.ip = (self.ip as isize + target as isize) as usize;
                    }
                }
                Some(chunk::OpCode::Jump) => {
                    let target = self.read_signed_16();
                    self.ip = (self.ip as isize + target as isize) as usize;
                }

                Some(chunk::OpCode::TestLess) => binary_op!(self, <, Number, Boolean, current_line),
                Some(chunk::OpCode::TestLessOrEqual) => {
                    binary_op!(self, <=, Number, Boolean, current_line)
                }
                Some(chunk::OpCode::TestGreater) => {
                    binary_op!(self, >, Number, Boolean, current_line)
                }
                Some(chunk::OpCode::TestGreaterOrEqual) => {
                    binary_op!(self, >=, Number, Boolean, current_line)
                }

                Some(chunk::OpCode::TestEqual) => {
                    let a = self.stack.pop(current_line)?;
                    let b = self.stack.pop(current_line)?;
                    self.stack.push(Value::Boolean(a == b));
                }
                Some(chunk::OpCode::TestNotEqual) => {
                    let a = self.stack.pop(current_line)?;
                    let b = self.stack.pop(current_line)?;
                    self.stack.push(Value::Boolean(a != b));
                }

                Some(chunk::OpCode::Index) => {
                    let the_value = self.stack.pop(current_line)?;
                    let indexer = self.stack.pop(current_line)?;
                    match indexer {
                        Value::String(s) => {
                            let v;
                            if let Value::Number(n) = the_value {
                                v = n as usize;
                            } else {
                                return runtime_error("Index must be number.", current_line);
                            }
                            // Todo: Make this better and maybe utf8 safe.
                            let c = s.into_bytes()[v];
                            self.stack.push(Value::Number(c as f64));
                        }

                        Value::ReferenceId(id) => {
                            let ref_type = &mut self.heap[id];
                            match ref_type {
                                ReferenceType::Array(ref mut a) => {
                                    let v = if let Value::Number(n) = the_value {
                                        n as usize
                                    } else {
                                        return runtime_error("Index must be number.", current_line);
                                    };
                                    if v >= a.len() {
                                        a.resize(v + 1, Value::Nil);
                                    }
                                    self.stack.push(a[v].clone());
                                }
                                ReferenceType::Map(m) => {
                                    let hashable_value =
                                        HashableValue::try_from(&the_value, current_line).unwrap();
                                    self.stack.push(
                                        m.get(&hashable_value).unwrap_or(&Value::Nil).clone(),
                                    );
                                }
                                _ => {
                                    return runtime_error(
                                        "Don't know how to index that.",
                                        current_line,
                                    );
                                }
                            }
                        }

                        _ => {
                            return runtime_error("Don't know how to index that.", current_line);
                        }
                    }
                }

                Some(chunk::OpCode::NewArray) => {
                    let id = self.new_reference_type(ReferenceType::Array(Vec::new()));
                    self.stack.push(Value::ReferenceId(id));
                }

                Some(chunk::OpCode::PushArray) => {
                    let value = self.stack.pop(current_line)?;
                    let array = self.stack.pop(current_line)?;
                    match array {
                        Value::ReferenceId(id) => {
                            let ref_type = &mut self.heap[id];
                            match ref_type {
                                ReferenceType::Array(ref mut a) => {
                                    a.push(value);
                                }
                                _ => {
                                    return runtime_error("Array push on non-array", current_line);
                                }
                            }
                            self.stack.push(Value::ReferenceId(id));
                        }
                        _ => {
                            return runtime_error("Array push on non-array", current_line);
                        }
                    }
                }

                Some(chunk::OpCode::IndexAssign) => {
                    let new_value = self.stack.pop(current_line)?;
                    let index_value = self.stack.pop(current_line)?;
                    let indexer = self.stack.pop(current_line)?;
                    match indexer {
                        Value::ReferenceId(id) => {
                            let ref_type = &mut self.heap[id];
                            match ref_type {
                                ReferenceType::Array(ref mut a) => {
                                    let n;
                                    if let Value::Number(value) = index_value {
                                        n = value as usize;
                                    } else {
                                        return Err(InterpreterError::RuntimeError(
                                            "Index must be number.".to_string(),
                                            current_line,
                                        ));
                                    }
                                    if n >= a.len() {
                                        a.resize(n + 1, Value::Nil);
                                    }
                                    a[n] = new_value;
                                }
                                ReferenceType::Map(ref mut m) => {
                                    m.insert(
                                        HashableValue::try_from(&index_value, current_line)?,
                                        new_value,
                                    );
                                }

                                _ => {
                                    return Err(InterpreterError::RuntimeError(
                                        "Don't know how to index assign that".to_string(),
                                        current_line,
                                    ));
                                }
                            }
                        }

                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "Don't know how to index assign that".to_string(),
                                current_line,
                            ));
                        }
                    }
                }

                Some(chunk::OpCode::BuiltinCall) => {
                    let builtin = self.stack.pop(current_line)?;
                    let callee = self.stack.pop(current_line)?;
                    let builtin = if let Value::String(s) = builtin {
                        s
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Expected builtin name".to_string(),
                            current_line,
                        ));
                    };

                    if builtin == "to_string" {
                        self.stack.push(Value::String(format!("{}", callee)));
                        continue;
                    }

                    // TODO: Some kind of data driven solution rather than hardcoded ifs.
                    match callee {
                        Value::ReferenceId(id) => {
                            let ref_type = &mut self.heap[id];
                            match ref_type {
                                ReferenceType::Array(ref mut a) => {
                                    if builtin == "len" {
                                        self.stack.push(Value::Number(a.len() as f64));
                                    } else if builtin == "push" {
                                        let value = self.stack.pop(current_line)?;
                                        a.push(value);
                                        self.stack.push(Value::Nil);
                                    } else if builtin == "pop" {
                                        self.stack.push(a.pop().unwrap());
                                    } else if builtin == "remove" {
                                        let to_remove = self.stack.pop(current_line)?;
                                        if let Value::Number(n) = to_remove {
                                            self.stack.push(a.remove(n as usize));
                                        } else {
                                            return Err(InterpreterError::RuntimeError(
                                                "Attempt to remove non-integer index from array"
                                                    .to_string(),
                                                current_line,
                                            ));
                                        }
                                    } else if builtin == "insert" {
                                        let to_insert_val = self.stack.pop(current_line)?;
                                        let to_insert = self.stack.pop(current_line)?;
                                        if let Value::Number(n) = to_insert {
                                            a.insert(n as usize, to_insert_val);
                                            self.stack.push(Value::Nil);
                                        } else {
                                            return Err(InterpreterError::RuntimeError(
                                                "Attempt to insert non-integer index from array"
                                                    .to_string(),
                                                current_line,
                                            ));
                                        }
                                    } else if builtin == "sort" {
                                        a.sort_by(|a, b| {
                                            HashableValue::try_from(a, current_line).unwrap().cmp(
                                                &HashableValue::try_from(b, current_line).unwrap(),
                                            )
                                        });
                                        self.stack.push(Value::ReferenceId(id));
                                    } else if builtin == "resize" {
                                        let v = self.stack.pop(current_line)?;
                                        let v = if let Value::Number(n) = v {
                                            n
                                        } else {
                                            panic!("Bad arg to array resize.");
                                        };
                                        a.resize(v as usize, Value::Nil);
                                        self.stack.push(Value::Nil);
                                    } else {
                                        return Err(InterpreterError::RuntimeError(
                                            "Unknown array builtin".to_string(),
                                            current_line,
                                        ));
                                    }
                                }
                                ReferenceType::External(ref mut e) => {
                                    let arity = e.get_arity(&builtin);
                                    let mut args = Vec::new();
                                    for _ in 0..arity {
                                        // Hack: copied pop

                                        args.push(self.stack.pop(current_line)?)
                                    }
                                    let rt = e.call(&builtin, args);
                                    if let ReferenceType::Nil = rt {
                                        self.stack.push(Value::Nil);
                                    } else {
                                        let id = self.new_reference_type(rt);
                                        self.stack.push(Value::ReferenceId(id));
                                    }
                                }
                                _ => {
                                    return Err(InterpreterError::RuntimeError(
                                        "Unknown builtin".to_string(),
                                        current_line,
                                    ))
                                }
                            }
                        }

                        Value::String(s) => {
                            if builtin == "len" {
                                self.stack.push(Value::Number(s.len() as f64));
                            } else if builtin == "readFile" {
                                self.stack
                                    .push(Value::String(std::fs::read_to_string(s).unwrap()));
                            } else if builtin == "split" {
                                let sep = self.stack.pop(current_line)?;
                                if let Value::String(sep) = sep {
                                    let parts: Vec<_> = s
                                        .split(&sep)
                                        .map(|p| Value::String(p.to_string()))
                                        .collect();
                                    let id = self.new_reference_type(ReferenceType::Array(parts));
                                    self.stack.push(Value::ReferenceId(id));
                                } else {
                                    return Err(InterpreterError::RuntimeError(
                                        "Expected string argument to split".to_string(),
                                        current_line,
                                    ));
                                }
                            } else if builtin == "parseNumber" {
                                self.stack.push(Value::Number(s.parse().unwrap()));
                            } else if builtin == "regex" {
                                let id = self.new_reference_type(ReferenceType::External(
                                    Box::new(regex::Regex::new(&s).unwrap()),
                                ));
                                self.stack.push(Value::ReferenceId(id));
                            } else {
                                return Err(InterpreterError::RuntimeError(
                                    "Unknown string builtin".to_string(),
                                    current_line,
                                ));
                            }
                        }

                        Value::Number(n) => {
                            if builtin == "floor" {
                                self.stack.push(Value::Number(n.floor()));
                            }
                        }

                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "Unknown builtin".to_string(),
                                current_line,
                            ))
                        }
                    }
                }

                Some(chunk::OpCode::MakeRange) => {
                    let right = if let Value::Number(n) = self.stack.pop(current_line)? {
                        n
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Expected number in range bounds".to_string(),
                            current_line,
                        ));
                    };
                    let left = if let Value::Number(n) = self.stack.pop(current_line)? {
                        n
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Expected number in range bounds".to_string(),
                            current_line,
                        ));
                    };
                    self.stack.push(Value::Range(left, right))
                }

                Some(chunk::OpCode::ForLoop) => {
                    let local_n = self.read_byte();
                    let jump_target = self.read_signed_16();
                    let target_ip = (self.ip as isize + jump_target as isize) as usize;
                    let range = self.stack.pop(current_line)?;
                    match range {
                        Value::Range(l, r) => {
                            if l < r {
                                self.locals[local_n as usize + self.locals_base] = Value::Number(l);
                                self.stack.push(Value::Range(l + 1.0, r));
                            } else {
                                self.ip = target_ip;
                            }
                        }
                        Value::ReferenceId(id) => {
                            let mut to_push = None;
                            {
                                let ref_type = &mut self.heap[id];
                                match ref_type {
                                    ReferenceType::Array(a) => {
                                        if a.len() > 0 {
                                            self.locals[local_n as usize + self.locals_base] =
                                                Value::Number(0.0);
                                            to_push = Some(Value::Range(1.0, a.len() as f64));
                                        } else {
                                            self.ip = target_ip;
                                        }
                                    }
                                    ReferenceType::Map(m) => {
                                        let keys: Vec<_> = m.keys().map(|e| e.clone()).collect();
                                        let len = keys.len();
                                        if len > 0 {
                                            self.locals[local_n as usize + self.locals_base] =
                                                Value::from(&keys[0]);
                                            to_push =
                                                Some(Value::MapForContext(keys, 1.0, len as f64));
                                        } else {
                                            self.ip = target_ip;
                                        }
                                    }
                                    _ => {
                                        return Err(InterpreterError::RuntimeError(
                                            "Don't know how to for over that".to_string(),
                                            current_line,
                                        ))
                                    }
                                }
                            }
                            if let Some(p) = to_push {
                                self.stack.push(p);
                            }
                        }
                        Value::MapForContext(keys, l, r) => {
                            let mut to_push = None;
                            if l < r {
                                self.locals[local_n as usize + self.locals_base] =
                                    Value::from(&keys[l as usize]);
                                to_push = Some(Value::MapForContext(keys, l + 1.0, r));
                            } else {
                                self.ip = (self.ip as isize + jump_target as isize) as usize;
                            }
                            if let Some(p) = to_push {
                                self.stack.push(p);
                            }
                        }
                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "Don't know how to for over that".to_string(),
                                current_line,
                            ))
                        }
                    }
                }

                Some(chunk::OpCode::PopMulti) => {
                    let n = self.read_byte();
                    self.stack.pop_multi(n as usize, current_line)?;
                }

                Some(chunk::OpCode::PushTrue) => {
                    self.stack.push(Value::Boolean(true));
                }

                Some(chunk::OpCode::PushFalse) => {
                    self.stack.push(Value::Boolean(false));
                }

                Some(chunk::OpCode::NewMap) => {
                    let id = self.new_reference_type(ReferenceType::Map(HashMap::new()));
                    self.stack.push(Value::ReferenceId(id));
                }

                Some(chunk::OpCode::PushMap) => {
                    let value = self.stack.pop(current_line)?;
                    let key = self.stack.pop(current_line)?;
                    let map = self.stack.pop(current_line)?;
                    if let Value::ReferenceId(id) = map {
                        let map = &mut self.heap[id];
                        if let ReferenceType::Map(ref mut m) = map {
                            m.insert(HashableValue::try_from(&key, current_line)?, value);
                        } else {
                            return Err(InterpreterError::RuntimeError(
                                "Map push on non-map".to_string(),
                                current_line,
                            ));
                        }
                        self.stack.push(Value::ReferenceId(id));
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Map push on non-map".to_string(),
                            current_line,
                        ));
                    }
                }

                Some(chunk::OpCode::Not) => {
                    let value = self.stack.pop(current_line)?;
                    self.stack.push(Value::Boolean(value.is_falsey()));
                }

                Some(chunk::OpCode::And) => {
                    let a = self.stack.pop(current_line)?;
                    let b = self.stack.pop(current_line)?;
                    self.stack
                        .push(Value::Boolean(!a.is_falsey() && !b.is_falsey()));
                }

                Some(chunk::OpCode::Dup) => {
                    let val = self.stack.peek();
                    self.stack.push(val);
                }

                Some(chunk::OpCode::JumpIfTrue) => {
                    let target = self.read_signed_16();
                    let value = self.stack.pop(current_line)?;
                    if value.is_truey() {
                        self.ip = (self.ip as isize + target as isize) as usize;
                    }
                }

                None => {
                    return Err(InterpreterError::RuntimeError(
                        "Bad instruction".to_string(),
                        current_line,
                    ))
                }
            }
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        self.ip += 1;
        self.chunk.code[self.ip - 1]
    }

    pub fn read_signed_byte(&mut self) -> i8 {
        self.read_byte() as i8
    }

    pub fn read_signed_16(&mut self) -> i16 {
        let number = self.read_byte() as usize;
        let number2 = self.read_byte() as usize;
        (number | number2 << 8) as i16
    }

    pub fn read_constant(&mut self) -> Value {
        let constant_number = self.read_byte();
        self.chunk.constants[constant_number as usize].clone()
    }

    pub fn new_reference_type(&mut self, value: ReferenceType) -> usize {
        self.heap.push(value);
        return self.heap.len() - 1;
    }
}
