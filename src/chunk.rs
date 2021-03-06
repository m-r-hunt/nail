use super::value::Value;
use std::collections::HashMap;

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

    NewArray = 22,
    PushArray = 23,

    IndexAssign = 24,

    BuiltinCall = 25,

    MakeRange = 26,
    ForLoop = 27,

    Remainder = 28,

    TestEqual = 29,
    TestNotEqual = 30,

    PopMulti = 31,

    PushTrue = 32,
    PushFalse = 33,

    NewMap = 34,
    PushMap = 35,

    Not = 36,

    Dup = 38,
    JumpIfTrue = 39,

    AssignGlobal = 40,
    LoadGlobal = 41,
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

            22 => Some(OpCode::NewArray),
            23 => Some(OpCode::PushArray),

            24 => Some(OpCode::IndexAssign),

            25 => Some(OpCode::BuiltinCall),

            26 => Some(OpCode::MakeRange),
            27 => Some(OpCode::ForLoop),

            28 => Some(OpCode::Remainder),

            29 => Some(OpCode::TestEqual),
            30 => Some(OpCode::TestNotEqual),

            31 => Some(OpCode::PopMulti),

            32 => Some(OpCode::PushTrue),
            33 => Some(OpCode::PushFalse),

            34 => Some(OpCode::NewMap),
            35 => Some(OpCode::PushMap),

            36 => Some(OpCode::Not),

            38 => Some(OpCode::Dup),

            39 => Some(OpCode::JumpIfTrue),

            40 => Some(OpCode::AssignGlobal),
            41 => Some(OpCode::LoadGlobal),

            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: Vec<Value>,
    pub globals: HashMap<String, Value>,
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
        (self.constants.len() - 1) as u8
    }

    pub fn register_function(&mut self, name: String, _arity: u8) {
        use std::collections::hash_map::Entry;
        if let Entry::Vacant(v) = self.function_names.entry(name) {
            v.insert(self.function_locations.len() as u8);
            self.function_locations.push(0);
        }
    }

    pub fn start_function(&mut self, name: &str, line: usize) -> usize {
        let address = self.code.len();
        self.code.push(OpCode::FunctionEntry as u8);
        self.lines.push(line);
        let ret = self.code.len();
        self.code.push(0);
        self.lines.push(line);
        self.function_locations[self.function_names[name] as usize] = address;
        ret
    }

    pub fn lookup_function(&self, name: &str) -> usize {
        let number = self.function_names[name];
        self.function_locations[number as usize]
    }

    pub fn register_global(&mut self, name: &str, value: Value) {
        self.globals.insert(name.to_string(), value);
    }

    pub fn check_global(&self, name: &str) -> bool {
        self.globals.get(name).is_some()
    }

    pub fn assign_global(&mut self, name: &str, value: Value) {
        self.globals.insert(name.to_string(), value);
    }
}
