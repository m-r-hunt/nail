use super::value::Value;

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum OpCode {
    Return = 1,

    Constant = 2,

    Negate = 3,

    Add = 4,
    Subtract = 5,
    Multiply = 6,
    Divide = 7,

    Print = 8,

    AssignLocal = 9,
    LoadLocal = 10,

    PushNil = 11,
    Pop = 12,

    FunctionEntry = 13,
    Call = 14,

    JumpIfFalse = 15,
    Jump = 16,

    TestLess = 17,
    TestLessOrEqual = 18,
    TestGreater = 19,
    TestGreaterOrEqual = 20,

    Index = 21,
}

impl OpCode {
    pub fn try_from(val: u8) -> Option<Self> {
        match val {
            1 => Some(OpCode::Return),

            2 => Some(OpCode::Constant),

            3 => Some(OpCode::Negate),

            4 => Some(OpCode::Add),
            5 => Some(OpCode::Subtract),
            6 => Some(OpCode::Multiply),
            7 => Some(OpCode::Divide),

            8 => Some(OpCode::Print),

            9 => Some(OpCode::AssignLocal),
            10 => Some(OpCode::LoadLocal),

            11 => Some(OpCode::PushNil),
            12 => Some(OpCode::Pop),

            13 => Some(OpCode::FunctionEntry),
            14 => Some(OpCode::Call),

            15 => Some(OpCode::JumpIfFalse),
            16 => Some(OpCode::Jump),

            17 => Some(OpCode::TestLess),
            18 => Some(OpCode::TestLessOrEqual),
            19 => Some(OpCode::TestGreater),
            20 => Some(OpCode::TestGreaterOrEqual),

            21 => Some(OpCode::Index),

            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: Vec<Value>,
    pub function_names: std::collections::HashMap<String, u8>,
    pub function_locations: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk::default()
    }

    pub fn write_chunk(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> u8 {
        self.constants.push(value);
        return (self.constants.len() - 1) as u8;
    }

    pub fn register_function(&mut self, name: String, _arity: u8) {
        use std::collections::hash_map::Entry;
        match self.function_names.entry(name) {
            Entry::Vacant(v) => {
                v.insert(self.function_locations.len() as u8);
                self.function_locations.push(0);
            }
            _ => {}
        };
    }

    pub fn start_function(&mut self, name: String, line: usize) -> usize {
        let address = self.code.len();
        self.code.push(OpCode::FunctionEntry as u8);
        self.lines.push(line);
        let ret = self.code.len();
        self.code.push(0);
        self.lines.push(line);
        self.function_locations[*self.function_names.get(&name).unwrap() as usize] = address;
        return ret;
    }

    pub fn lookup_function(&self, name: &str) -> usize {
        let number = self.function_names.get(name).unwrap();
        return self.function_locations[*number as usize];
    }
}
