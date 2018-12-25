use crate::core::instruction::Instruction;
use crate::core::instruction::SRType;
use crate::core::operation::{decode_imm_shift, thumb_expand_imm};
use crate::core::register::Reg;
use bit_field::BitField;

#[allow(non_snake_case)]
#[inline]
pub fn decode_CMP_imm_t1(opcode: u16) -> Instruction {
    Instruction::CMP_imm {
        rn: Reg::from(opcode.get_bits(8..11) as u8),
        imm32: u32::from(opcode.get_bits(0..8)),
        thumb32: false,
    }
}

#[allow(non_snake_case)]
#[inline]
pub fn decode_CMP_imm_t2(opcode: u32) -> Instruction {
    let rn: u8 = opcode.get_bits(16..20) as u8;

    let imm3: u8 = opcode.get_bits(12..15) as u8;
    let imm8: u8 = opcode.get_bits(0..8) as u8;
    let i: u8 = opcode.get_bit(26) as u8;

    let params = [i, imm3, imm8];
    let lengths = [1, 3, 8];

    Instruction::CMP_imm {
        rn: Reg::from(rn),
        imm32: thumb_expand_imm(&params, &lengths),
        thumb32: true,
    }
}

#[allow(non_snake_case)]
#[inline]
pub fn decode_CMP_reg_t1(opcode: u16) -> Instruction {
    Instruction::CMP_reg {
        rn: Reg::from(opcode.get_bits(0..3) as u8),
        rm: Reg::from(opcode.get_bits(3..6) as u8),
        shift_t: SRType::LSL,
        shift_n: 0,
        thumb32: false,
    }
}

#[allow(non_snake_case)]
#[inline]
pub fn decode_CMP_reg_t2(opcode: u16) -> Instruction {
    Instruction::CMP_reg {
        rn: Reg::from(((opcode.get_bit(7) as u8) << 3) + opcode.get_bits(0..3) as u8),
        rm: Reg::from(opcode.get_bits(3..7) as u8),
        shift_t: SRType::LSL,
        shift_n: 0,
        thumb32: false,
    }
}

#[allow(non_snake_case)]
pub fn decode_CMP_reg_t3(opcode: u32) -> Instruction {
    let imm3: u8 = opcode.get_bits(12..15) as u8;
    let imm2: u8 = opcode.get_bits(6..8) as u8;
    let type_: u8 = opcode.get_bits(4..6) as u8;

    let (shift_t, shift_n) = decode_imm_shift(type_, (imm3 << 2) + imm2);
    Instruction::CMP_reg {
        rm: Reg::from(opcode.get_bits(0..4)),
        rn: Reg::from(opcode.get_bits(16..20)),
        shift_t: shift_t,
        shift_n: shift_n,
        thumb32: true,
    }
}
