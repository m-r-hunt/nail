use super::chunk::*;

pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut i = 0;
    while i < chunk.code.len() {
        i = disassemble_instruction(chunk, i);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    let instr = chunk.code[offset];
    print!("{:04x} ", offset);
    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.lines[offset]);
    }

    match OpCode::try_from(instr) {
        Some(OpCode::Return) => simple_instruction("OP_RETURN", offset),

        Some(OpCode::Constant) => constant_instruction("OP_CONSTANT", &chunk, offset),

        Some(OpCode::Negate) => simple_instruction("OP_NEGATE", offset),

        Some(OpCode::Add) => simple_instruction("OP_ADD", offset),
        Some(OpCode::Subtract) => simple_instruction("OP_SUBTRACT", offset),
        Some(OpCode::Multiply) => simple_instruction("OP_MULTIPLY", offset),
        Some(OpCode::Divide) => simple_instruction("OP_DIVIDE", offset),

        Some(OpCode::Print) => simple_instruction("OP_PRINT", offset),

        Some(OpCode::AssignLocal) => number_instruction("OP_ASSIGN_LOCAL", &chunk, offset),
        Some(OpCode::LoadLocal) => number_instruction("OP_LOAD_LOCAL", &chunk, offset),

        Some(OpCode::PushNil) => simple_instruction("OP_PUSH_NIL", offset),
        Some(OpCode::Pop) => simple_instruction("OP_POP", offset),

        Some(OpCode::FunctionEntry) => number_instruction("OP_FN_ENTRY", &chunk, offset),
        Some(OpCode::Call) => number_instruction("OP_CALL", &chunk, offset),

        Some(OpCode::JumpIfFalse) => {
            signed_number_16_instruction("OP_JUMP_IF_FALSE", &chunk, offset)
        }
        Some(OpCode::Jump) => signed_number_16_instruction("OP_JUMP", &chunk, offset),

        Some(OpCode::TestLess) => simple_instruction("OP_TEST_LESS", offset),
        Some(OpCode::TestLessOrEqual) => simple_instruction("OP_TEST_LESS_OR_EQUAL", offset),
        Some(OpCode::TestGreater) => simple_instruction("OP_TEST_GREATER", offset),
        Some(OpCode::TestGreaterOrEqual) => simple_instruction("OP_TEST_GREATER_OR_EQUAL", offset),

        Some(OpCode::Index) => simple_instruction("OP_INDEX", offset),

        Some(OpCode::NewArray) => simple_instruction("OP_NEW_ARRAY", offset),
        Some(OpCode::PushArray) => simple_instruction("OP_PUSH_ARRAY", offset),

        Some(OpCode::IndexAssign) => simple_instruction("OP_INDEX_ASSIGN", offset),

        Some(OpCode::BuiltinCall) => simple_instruction("OP_BUILTIN_CALL", offset),

        Some(OpCode::MakeRange) => simple_instruction("OP_MAKE_RANGE", offset),
        Some(OpCode::ForLoop) => for_instruction(&chunk, offset),

        Some(OpCode::Remainder) => simple_instruction("OP_REMAINDER", offset),

        Some(OpCode::TestEqual) => simple_instruction("OP_TEST_EQUAL", offset),
        Some(OpCode::TestNotEqual) => simple_instruction("OP_TEST_Not_EQUAL", offset),

        Some(OpCode::PopMulti) => number_instruction("OP_POP_MULTI", &chunk, offset),

        Some(OpCode::PushTrue) => simple_instruction("OP_PUSH_TRUE", offset),
        Some(OpCode::PushFalse) => simple_instruction("OP_PUSH_FALSE", offset),

        Some(OpCode::NewMap) => simple_instruction("OP_NEW_MAP", offset),
        Some(OpCode::PushMap) => simple_instruction("OP_PUSH_MAP", offset),

        Some(OpCode::Not) => simple_instruction("OP_NOT", offset),
        Some(OpCode::And) => simple_instruction("OP_AND", offset),

        Some(OpCode::Dup) => simple_instruction("OP_DUP", offset),

        Some(OpCode::JumpIfTrue) => {
            signed_number_16_instruction("OP_JUMP_IF_TRUE", &chunk, offset)
        }

        None => {
            println!("Unknown opcode {}", instr);
            offset + 1
        }
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    return offset + 1;
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant = chunk.code[offset + 1];
    println!(
        "{} {} '{}'",
        name, constant, chunk.constants[constant as usize]
    );
    return offset + 2;
}

fn number_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let number = chunk.code[offset + 1];
    println!("{} {}", name, number);
    return offset + 2;
}

fn signed_number_16_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let number = chunk.code[offset + 1];
    let number2 = chunk.code[offset + 2];
    println!(
        "{} {}",
        name,
        (number as usize | (number2 as usize) << 8) as i16
    );
    return offset + 3;
}

fn for_instruction(chunk: &Chunk, offset: usize) -> usize {
    let local = chunk.code[offset + 1];
    let jump_target = chunk.code[offset + 2];
    let jump_target2 = chunk.code[offset + 3];
    println!(
        "OP_FOR_LOOP l={} jt={}",
        local,
        (jump_target as usize | (jump_target2 as usize) << 8) as i16
    );
    return offset + 4;
}
