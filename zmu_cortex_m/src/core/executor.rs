use bit_field::BitField;
use bus::Bus;
use core::fault::Fault;
use core::instruction::{CpsEffect, Instruction, SRType};
use core::operation::{add_with_carry, decode_imm_shift, shift_c, sign_extend};
use core::register::{Apsr, Ipsr, Reg, SpecialReg};
use core::Core;
use semihosting::decode_semihostcmd;
use semihosting::semihost_return;
use semihosting::SemihostingCommand;
use semihosting::SemihostingResponse;

pub enum ExecuteResult {
    // Instruction execution resulted in a fault.
    Fault { fault: Fault },
    // The instruction was taken normally
    Taken { cycles: u64 },
    // The instruction was not taken as the condition did not pass
    NotTaken,
    // The execution branched to a new address, pc was set accordingly
    Branched { cycles: u64 },
}

#[allow(unused_variables)]
pub fn execute<T: Bus, F>(
    mut core: &mut Core<T>,
    instruction: &Instruction,
    mut semihost_func: F,
) -> ExecuteResult
where
    F: FnMut(&SemihostingCommand) -> SemihostingResponse,
{
    match *instruction {
        Instruction::ADC_reg {
            ref rn,
            ref rd,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let r_m = core.get_r(rm);
                let (result, carry, overflow) = add_with_carry(r_n, r_m, core.psr.get_c());

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                    core.psr.set_v(overflow);
                }

                core.set_r(rd, result);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::ASR_imm {
            ref rd,
            ref rm,
            ref imm5,
            ref setflags,
        } => {
            if core.condition_passed() {
                let (_, shift_n) = decode_imm_shift(0b10, *imm5);

                let (result, carry) = shift_c(
                    core.get_r(rm),
                    SRType::ASR,
                    usize::from(shift_n),
                    core.psr.get_c(),
                );

                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::ASR_reg {
            ref rd,
            ref rm,
            ref rn,
            ref setflags,
        } => {
            if core.condition_passed() {
                let shift_n = core.get_r(rm).get_bits(0..8);
                let (result, carry) = shift_c(
                    core.get_r(rn),
                    SRType::ASR,
                    shift_n as usize,
                    core.psr.get_c(),
                );
                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::BIC_reg {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let result = core.get_r(rn) & (core.get_r(rm) ^ 0xffff_ffff);
                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::CPS { ref im } => {
            if im == &CpsEffect::IE {
                core.primask = false;
            } else {
                core.primask = true;
            }
            return ExecuteResult::Taken { cycles: 1 };
        }
        Instruction::CBZ {
            ref rn,
            ref nonzero,
            ref imm32,
        } => {
            if nonzero ^ (core.get_r(rn) == 0) {
                let pc = core.get_r(&Reg::PC);
                core.branch_write_pc(pc + imm32);
                return ExecuteResult::Branched { cycles: 1 };
            } else {
                return ExecuteResult::Taken { cycles: 1 };
            }
        }
        Instruction::DMB => {
            if core.condition_passed() {
                return ExecuteResult::Taken { cycles: 4 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::DSB => {
            if core.condition_passed() {
                return ExecuteResult::Taken { cycles: 4 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::ISB => {
            if core.condition_passed() {
                return ExecuteResult::Taken { cycles: 4 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::IT {
            ref x,
            ref y,
            ref z,
            ref firstcond,
            ref mask,
        } => {
            core.set_itstate((((firstcond.value() as u32) << 4) + *mask as u32) as u8);
            return ExecuteResult::Taken { cycles: 4 };
        }

        Instruction::MRS {
            ref rd,
            ref spec_reg,
        } => {
            if core.condition_passed() {
                match spec_reg {
                    //APSR => {core.set_r(rd, core.psr.value & 0xf000_0000),
                    &SpecialReg::IPSR => {
                        let ipsr_val = core.psr.get_exception_number() as u32;
                        core.set_r(rd, ipsr_val);
                    }
                    //MSP => core.set_r(rd, core.get_r(Reg::MSP)),
                    //PSP => core.set_r(rd, core.get_r(Reg::PSP),
                    &SpecialReg::PRIMASK => {
                        let primask = core.primask as u32;
                        core.set_r(rd, primask);
                    }
                    //CONTROL => core.set_r(rd,core.control as u32),
                    _ => panic!("unsupported MRS operation"),
                }
                return ExecuteResult::Taken { cycles: 4 };
            }

            ExecuteResult::NotTaken
        }
        Instruction::MSR_reg {
            ref rn,
            ref spec_reg,
        } => {
            if core.condition_passed() {
                match spec_reg {
                    //APSR => {core.set_r(rd, core.psr.value & 0xf000_0000),
                /*&SpecialReg::IPSR => {
                    let ipsr_val = core.psr.get_exception_number() as u32;
                    core.set_r(rd, ipsr_val);
                }*/
                    &SpecialReg::MSP => {
                        let msp = core.get_r(rn);
                        core.set_msp(msp);
                    }
                    &SpecialReg::PSP => {
                        let psp = core.get_r(rn);
                        core.set_psp(psp);
                    }
                    //PSP => core.set_r(rd, core.get_r(Reg::PSP),
                    &SpecialReg::PRIMASK => {
                        let primask = core.get_r(rn) & 1 == 1;
                        core.primask = primask;
                    }
                    //CONTROL => core.set_r(rd,core.control as u32),
                    _ => panic!("unsupported MSR operation"),
                }
                return ExecuteResult::Taken { cycles: 4 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::MOV_reg {
            ref rd,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let result = core.get_r(rm);
                core.set_r(rd, result);

                if *rd != Reg::PC {
                    if *setflags {
                        core.psr.set_n(result);
                        core.psr.set_z(result);
                    }
                    return ExecuteResult::Taken { cycles: 1 };
                } else {
                    unimplemented!()
                }
            }

            ExecuteResult::NotTaken
        }
        Instruction::LSL_imm {
            ref rd,
            ref rm,
            ref imm5,
            ref setflags,
        } => {
            if core.condition_passed() {
                let (_, shift_n) = decode_imm_shift(0b00, *imm5);
                let (result, carry) = shift_c(
                    core.get_r(rm),
                    SRType::LSL,
                    shift_n as usize,
                    core.psr.get_c(),
                );
                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LSL_reg {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let shift_n = core.get_r(rm).get_bits(0..8);
                let (result, carry) = shift_c(
                    core.get_r(rn),
                    SRType::LSL,
                    shift_n as usize,
                    core.psr.get_c(),
                );
                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LSR_imm {
            ref rd,
            ref rm,
            ref imm5,
            ref setflags,
        } => {
            if core.condition_passed() {
                let (_, shift_n) = decode_imm_shift(0b01, *imm5);
                let (result, carry) = shift_c(
                    core.get_r(rm),
                    SRType::LSR,
                    usize::from(shift_n),
                    core.psr.get_c(),
                );
                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LSR_reg {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let shift_n = core.get_r(rm).get_bits(0..8);
                let (result, carry) = shift_c(
                    core.get_r(rn),
                    SRType::LSR,
                    shift_n as usize,
                    core.psr.get_c(),
                );
                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }

            ExecuteResult::NotTaken
        }

        Instruction::BL { imm32 } => {
            if core.condition_passed() {
                let pc = core.get_r(&Reg::PC);
                core.set_r(&Reg::LR, pc | 0x01);
                let target = ((pc as i32) + imm32) as u32;
                core.branch_write_pc(target);
                return ExecuteResult::Branched { cycles: 4 };
            }

            ExecuteResult::NotTaken
        }

        Instruction::BKPT { imm32 } => {
            if imm32 == 0xab {
                let r0 = core.get_r(&Reg::R0);
                let r1 = core.get_r(&Reg::R1);
                let semihost_cmd = decode_semihostcmd(r0, r1, &mut core);
                let semihost_response = semihost_func(&semihost_cmd);
                semihost_return(&mut core, &semihost_response);
            }
            return ExecuteResult::Taken { cycles: 1 };
        }

        Instruction::NOP => {
            return ExecuteResult::Taken { cycles: 1 };
        }

        Instruction::MUL {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let operand1 = core.get_r(rn);
                let operand2 = core.get_r(rm);

                let result = operand1.wrapping_mul(operand2);

                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::ORR_reg {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let r_m = core.get_r(rm);

                let result = r_n | r_m;

                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::ORR_imm {
            ref rd,
            ref rn,
            ref imm32,
            ref setflags,
        } => unimplemented!(),

        Instruction::EOR_reg {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let r_m = core.get_r(rm);

                let result = r_n ^ r_m;

                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }

            ExecuteResult::NotTaken
        }

        Instruction::AND_reg {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let r_m = core.get_r(rm);

                let result = r_n & r_m;

                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::BX { ref rm } => {
            if core.condition_passed() {
                let r_m = core.get_r(rm);
                core.bx_write_pc(r_m);
                return ExecuteResult::Branched { cycles: 3 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::BLX { ref rm } => {
            if core.condition_passed() {
                let pc = core.get_r(&Reg::PC);
                let target = core.get_r(rm);
                core.set_r(&Reg::LR, (((pc - 2) >> 1) << 1) | 1);
                core.blx_write_pc(target);
                return ExecuteResult::Branched { cycles: 3 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDM {
            ref registers,
            ref rn,
        } => {
            if core.condition_passed() {
                let regs_size = 4 * (registers.len() as u32);

                let mut address = core.get_r(rn);

                let mut branched = false;
                for reg in registers.iter() {
                    let value = core.bus.read32(address);
                    if &reg == &Reg::PC {
                        core.load_write_pc(value);
                        branched = true;
                    } else {
                        core.set_r(&reg, value);
                    }
                    address += 4;
                }

                if !registers.contains(rn) {
                    core.add_r(rn, regs_size);
                }
                let cc = 1 + registers.len() as u64;
                if branched {
                    return ExecuteResult::Branched { cycles: cc };
                }
                return ExecuteResult::Taken { cycles: cc };
            }
            ExecuteResult::NotTaken
        }
        Instruction::MOV_imm {
            ref rd,
            imm32,
            setflags,
        } => {
            if core.condition_passed() {
                let result = imm32 as u32;
                core.set_r(rd, result);
                if setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::MVN_reg {
            ref rd,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let result = core.get_r(rm) ^ 0xFFFF_FFFF;
                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::MVN_imm {
            ref rd,
            ref imm32,
            ref setflags,
        } => unimplemented!(),

        Instruction::B { ref cond, imm32 } => if core.condition_passed_b(cond) {
            let pc = core.get_r(&Reg::PC);
            let target = ((pc as i32) + imm32) as u32;
            core.branch_write_pc(target);
            return ExecuteResult::Branched { cycles: 3 };
        } else {
            ExecuteResult::NotTaken
        },

        Instruction::CMP_imm { ref rn, imm32 } => {
            if core.condition_passed() {
                let (result, carry, overflow) =
                    add_with_carry(core.get_r(rn), imm32 ^ 0xFFFF_FFFF, true);
                core.psr.set_n(result);
                core.psr.set_z(result);
                core.psr.set_c(carry);
                core.psr.set_v(overflow);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::CMP_reg { ref rn, ref rm } => {
            if core.condition_passed() {
                let (result, carry, overflow) =
                    add_with_carry(core.get_r(rn), core.get_r(rm) ^ 0xFFFF_FFFF, true);
                core.psr.set_n(result);
                core.psr.set_z(result);
                core.psr.set_c(carry);
                core.psr.set_v(overflow);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::CMN_reg { ref rn, ref rm } => {
            if core.condition_passed() {
                let (result, carry, overflow) =
                    add_with_carry(core.get_r(rn), core.get_r(rm), false);
                core.psr.set_n(result);
                core.psr.set_z(result);
                core.psr.set_c(carry);
                core.psr.set_v(overflow);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::PUSH { ref registers, thumb32 } => {
            if core.condition_passed() {
                let regs_size = 4 * (registers.len() as u32);
                let sp = core.get_r(&Reg::SP);
                let mut address = sp - regs_size;

                for reg in registers.iter() {
                    let value = core.get_r(&reg);
                    core.bus.write32(address, value);
                    address += 4;
                }

                core.set_r(&Reg::SP, sp - regs_size);
                return ExecuteResult::Taken {
                    cycles: 1 + registers.len() as u64,
                };
            }
            ExecuteResult::NotTaken
        }

        Instruction::POP { ref registers } => {
            if core.condition_passed() {
                let regs_size = 4 * (registers.len() as u32);
                let sp = core.get_r(&Reg::SP);
                let mut address = sp;

                for reg in registers.iter() {
                    if reg == Reg::PC {
                        let target = core.bus.read32(address);
                        core.bx_write_pc(target);
                    } else {
                        let value = core.bus.read32(address);
                        core.set_r(&reg, value);
                    }
                    address += 4;
                }

                core.set_r(&Reg::SP, sp + regs_size);
                if registers.contains(&Reg::PC) {
                    return ExecuteResult::Branched {
                        cycles: 4 + registers.len() as u64,
                    };
                } else {
                    return ExecuteResult::Taken {
                        cycles: 1 + registers.len() as u64,
                    };
                }
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDR_imm {
            ref rt,
            ref rn,
            imm32,
            index,
            add,
            wback,
            thumb32,
        } => {
            if core.condition_passed() {
                let offset_address = if add {
                    core.get_r(rn) + imm32
                } else {
                    core.get_r(rn) - imm32
                };

                let address = if index {
                    offset_address
                } else {
                    core.get_r(rn)
                };

                let data = core.bus.read32(address);
                if wback {
                    core.set_r(rn, offset_address);
                }

                if rt == &Reg::PC {
                    core.load_write_pc(data);
                    return ExecuteResult::Branched { cycles: 1 };
                } else {
                    core.set_r(rt, data);
                    return ExecuteResult::Taken { cycles: 1 };
                }
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDR_reg {
            ref rt,
            ref rn,
            ref rm,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + core.get_r(rm);
                let value = core.bus.read32(address);

                if rt == &Reg::PC {
                    core.load_write_pc(value);
                    return ExecuteResult::Branched { cycles: 2 };
                } else {
                    core.set_r(rt, value);
                    return ExecuteResult::Taken { cycles: 2 };
                }
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDRB_imm {
            ref rt,
            ref rn,
            imm32,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + imm32;
                let value = u32::from(core.bus.read8(address));
                core.set_r(rt, value);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDRB_reg {
            ref rt,
            ref rn,
            ref rm,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + core.get_r(rm);
                let value = u32::from(core.bus.read8(address));
                core.set_r(rt, value);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDRH_imm {
            ref rt,
            ref rn,
            imm32,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + imm32;
                let value = u32::from(core.bus.read16(address));
                core.set_r(rt, value);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDRH_reg {
            ref rt,
            ref rn,
            ref rm,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + core.get_r(rm);
                let value = u32::from(core.bus.read16(address));
                core.set_r(rt, value);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDRSH_reg {
            ref rt,
            ref rn,
            ref rm,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + core.get_r(rm);
                let data = u32::from(core.bus.read16(address));
                core.set_r(rt, sign_extend(data, 15, 32) as u32);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDRSB_reg {
            ref rt,
            ref rn,
            ref rm,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + core.get_r(rm);
                let data = u32::from(core.bus.read8(address));
                core.set_r(rt, sign_extend(data, 7, 32) as u32);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::SBC_reg {
            ref rn,
            ref rd,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let r_m = core.get_r(rm);
                let (result, carry, overflow) =
                    add_with_carry(r_n, r_m ^ 0xffff_ffff, core.psr.get_c());

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                    core.psr.set_v(overflow);
                }

                core.set_r(rd, result);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::STM {
            ref registers,
            ref rn,
            wback,
        } => {
            if core.condition_passed() {
                let regs_size = 4 * (registers.len() as u32);

                let mut address = core.get_r(rn);

                for reg in registers.iter() {
                    let r = core.get_r(&reg);
                    core.bus.write32(address, r);
                    address += 4;
                }

                if wback {
                    core.add_r(rn, regs_size);
                }
                return ExecuteResult::Taken {
                    cycles: 1 + registers.len() as u64,
                };
            }
            ExecuteResult::NotTaken
        }

        Instruction::STR_imm {
            ref rt,
            ref rn,
            imm32,
            index,
            add,
            wback,
            thumb32,
        } => {
            if core.condition_passed() {
                let offset_address = if add {
                    core.get_r(rn) + imm32
                } else {
                    core.get_r(rn) - imm32
                };

                let address = if index {
                    offset_address
                } else {
                    core.get_r(rn)
                };

                let value = core.get_r(rt);
                if wback {
                    core.set_r(rn, offset_address);
                }

                core.bus.write32(address, value);

                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::STR_reg {
            ref rt,
            ref rn,
            ref rm,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + core.get_r(rm);
                let value = core.get_r(rt);
                core.bus.write32(address, value);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::STRB_reg {
            ref rt,
            ref rn,
            ref rm,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + core.get_r(rm);
                let value = core.get_r(rt);
                core.bus.write8(address, value.get_bits(0..8) as u8);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::STRB_imm {
            ref rt,
            ref rn,
            imm32,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + imm32;
                let value = core.get_r(rt);
                core.bus.write8(address, value.get_bits(0..8) as u8);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::STRH_imm {
            ref rt,
            ref rn,
            imm32,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + imm32;
                let value = core.get_r(rt);
                core.bus.write16(address, value.get_bits(0..16) as u16);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::STRH_reg {
            ref rt,
            ref rn,
            ref rm,
        } => {
            if core.condition_passed() {
                let address = core.get_r(rn) + core.get_r(rm);
                let value = core.get_r(rt);
                core.bus.write16(address, value.get_bits(0..16) as u16);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::LDR_lit {
            ref rt,
            imm32,
            thumb32,
        } => {
            if core.condition_passed() {
                let base = core.get_r(&Reg::PC) & 0xffff_fffc;
                let value = core.bus.read32(base + imm32);
                core.set_r(rt, value);
                return ExecuteResult::Taken { cycles: 2 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::ADD_reg {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let (result, carry, overflow) =
                    add_with_carry(core.get_r(rn), core.get_r(rm), false);

                if rd == &Reg::PC {
                    core.branch_write_pc(result);
                    return ExecuteResult::Branched { cycles: 3 };
                } else {
                    if *setflags {
                        core.psr.set_n(result);
                        core.psr.set_z(result);
                        core.psr.set_c(carry);
                        core.psr.set_v(overflow);
                    }
                    core.set_r(rd, result);
                    return ExecuteResult::Taken { cycles: 1 };
                }
            } else {
                ExecuteResult::NotTaken
            }
        }

        Instruction::ADD_imm {
            ref rn,
            ref rd,
            imm32,
            ref setflags,
            thumb32
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let (result, carry, overflow) = add_with_carry(r_n, imm32, false);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                    core.psr.set_v(overflow);
                }

                core.set_r(rd, result);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::ADR { ref rd, imm32 } => {
            if core.condition_passed() {
                let result = (core.get_r(&Reg::PC) & 0xffff_fffc) + imm32;
                core.set_r(rd, result);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::RSB_imm {
            ref rd,
            ref rn,
            imm32,
            ref setflags,
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let (result, carry, overflow) = add_with_carry(r_n ^ 0xFFFF_FFFF, imm32, true);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                    core.psr.set_v(overflow);
                }

                core.set_r(rd, result);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::SUB_imm {
            ref rn,
            ref rd,
            imm32,
            ref setflags,
            thumb32,
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let (result, carry, overflow) = add_with_carry(r_n, imm32 ^ 0xFFFF_FFFF, true);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                    core.psr.set_v(overflow);
                }

                core.set_r(rd, result);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::SUB_reg {
            ref rn,
            ref rd,
            ref rm,
            ref setflags,
            ref shift_t,
            ref shift_n,
            thumb32,
        } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let r_m = core.get_r(rm);
                let (result, carry, overflow) = add_with_carry(r_n, r_m ^ 0xFFFF_FFFF, true);
                core.set_r(rd, result);

                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                    core.psr.set_v(overflow);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::TBB { ref rn, ref rm } => {
            if core.condition_passed() {
                let r_n = core.get_r(rn);
                let r_m = core.get_r(rm);
                let pc = core.get_r(&Reg::PC);
                let halfwords = u32::from(core.bus.read8(r_n + r_m));

                core.branch_write_pc(pc + 2*halfwords);

                return ExecuteResult::Branched { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::TST_reg { ref rn, ref rm } => {
            if core.condition_passed() {
                let result = core.get_r(rn) & core.get_r(rm);

                core.psr.set_n(result);
                core.psr.set_z(result);
                //core.psr.set_c(carry); carry = shift_c()
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::UXTB { ref rd, ref rm } => {
            if core.condition_passed() {
                let rotated = core.get_r(rm);
                core.set_r(rd, rotated.get_bits(0..8));
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::UXTH { ref rd, ref rm } => {
            if core.condition_passed() {
                let rotated = core.get_r(rm);
                core.set_r(rd, rotated.get_bits(0..16));
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::SXTB { ref rd, ref rm } => {
            if core.condition_passed() {
                let rotated = core.get_r(rm);
                core.set_r(rd, sign_extend(rotated.get_bits(0..8), 7, 32) as u32);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        Instruction::SXTH { ref rd, ref rm } => {
            if core.condition_passed() {
                let rotated = core.get_r(rm);
                core.set_r(rd, sign_extend(rotated.get_bits(0..16), 15, 32) as u32);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::REV { ref rd, ref rm } => {
            if core.condition_passed() {
                let rm_ = core.get_r(rm);
                core.set_r(
                    rd,
                    ((rm_ & 0xff) << 24)
                        + ((rm_ & 0xff00) << 8)
                        + ((rm_ & 0xff_0000) >> 8)
                        + ((rm_ & 0xff00_0000) >> 24),
                );
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::REV16 { ref rd, ref rm } => {
            if core.condition_passed() {
                let rm_ = core.get_r(rm);
                core.set_r(
                    rd,
                    ((rm_ & 0xff) << 8)
                        + ((rm_ & 0xff00) >> 8)
                        + ((rm_ & 0xff_0000) << 8)
                        + ((rm_ & 0xff00_0000) >> 8),
                );
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::REVSH { ref rd, ref rm } => {
            if core.condition_passed() {
                let rm_ = core.get_r(rm);
                core.set_r(
                    rd,
                    ((sign_extend(rm_ & 0xff, 7, 24) as u32) << 8) + ((rm_ & 0xff00) >> 8),
                );
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::ROR_reg {
            ref rd,
            ref rn,
            ref rm,
            ref setflags,
        } => {
            if core.condition_passed() {
                let shift_n = core.get_r(rm) & 0xff;
                let (result, carry) = shift_c(
                    core.get_r(rn),
                    SRType::ROR,
                    shift_n as usize,
                    core.psr.get_c(),
                );
                core.set_r(rd, result);
                if *setflags {
                    core.psr.set_n(result);
                    core.psr.set_z(result);
                    core.psr.set_c(carry);
                }
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::SVC { ref imm32 } => {
            if core.condition_passed() {
                println!("SVC {}", imm32);
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::SEV => {
            if core.condition_passed() {
                println!("SEV");
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::WFE => {
            if core.condition_passed() {
                //TODO
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::WFI => {
            if core.condition_passed() {
                //TODO
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }
        Instruction::YIELD => {
            if core.condition_passed() {
                println!("YIELD");
                return ExecuteResult::Taken { cycles: 1 };
            }
            ExecuteResult::NotTaken
        }

        // ARMv7-M
        Instruction::MCR {
            ref rt,
            ref coproc,
            ref opc1,
            ref opc2,
            ref crn,
            ref crm,
        } => unimplemented!(),

        // ARMv7-M
        Instruction::MCR2 {
            ref rt,
            ref coproc,
            ref opc1,
            ref opc2,
            ref crn,
            ref crm,
        } => unimplemented!(),

        // ARMv7-M
        Instruction::LDC_imm {
            ref coproc,
            ref imm32,
            ref crd,
            ref rn,
        } => unimplemented!(),

        // ARMv7-M
        Instruction::LDC2_imm {
            ref coproc,
            ref imm32,
            ref crd,
            ref rn,
        } => unimplemented!(),

        // ARMv7-M
        Instruction::UDIV {
            ref rd,
            ref rn,
            ref rm,
        } => unimplemented!(),

        // ARMv7-M
        Instruction::UMLAL {
            ref rdlo,
            ref rdhi,
            ref rn,
            ref rm,
        } => unimplemented!(),

        // ARMv7-M
        Instruction::SMLAL {
            ref rdlo,
            ref rdhi,
            ref rn,
            ref rm,
        } => unimplemented!(),

        Instruction::UDF {
            ref imm32,
            ref opcode,
        } => {
            println!("UDF {}, {}", imm32, opcode);
            panic!("undefined");
            //Some(Fault::UndefinedInstruction)
        }
    }
}
