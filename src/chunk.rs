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

            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: Vec<Value>,
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
}
