use crate::core::instruction::Instruction;
use crate::core::operation::get_reglist;
use crate::core::register::Reg;
use crate::core::ThumbCode;
use bit_field::BitField;

#[allow(non_snake_case)]
#[inline]
pub fn decode_PUSH_t1(opcode: u16) -> Instruction {
    let mut regs = get_reglist(opcode & 0b_1111_1111);

    if opcode.get_bit(8) {
        regs.insert(Reg::LR);
    }

    Instruction::PUSH {
        registers: regs,
        thumb32: false,
    }
}

#[allow(non_snake_case)]
pub fn decode_PUSH_t2(opcode: u32) -> Instruction {
    let regs = get_reglist((opcode & 0b0101_1111_1111_1111) as u16);

    Instruction::PUSH {
        registers: regs,
        thumb32: true,
    }
}

#[allow(non_snake_case)]
pub fn decode_PUSH_t3(opcode: u32) -> Instruction {
    Instruction::UDF {
        imm32: 0,
        opcode: ThumbCode::from(opcode),
    }
}
