use core::ops::Range;

pub trait Bits {
    fn get_bits(&self, range: Range<usize>) -> Self;
    fn get_bit(&self, bit: usize) -> bool;
    fn set_bit(&mut self, bit: usize, value: bool);

    /// 0) count = (range.end - range.start)
    /// 1) set lowest count bits to 1
    ///   (1 << count) - 1                               |  0000 1111
    /// 2) left shift by range.start                     |  0001 1110
    /// 3) invert                                        |  1110 0001
    fn set_bits(&mut self, range: Range<usize>, value: Self);
}

impl Bits for u32 {
    #[inline(always)]
    fn get_bits(&self, range: Range<usize>) -> Self {
        let bits = *self << (32 - range.end) >> (32 - range.end);
        bits >> range.start
    }
    #[inline(always)]
    fn get_bit(&self, bit: usize) -> bool {
        (*self & 1 << bit) == 1 << bit
    }

    #[inline(always)]
    fn set_bits(&mut self, range: Range<usize>, value: Self) {
        let mask: Self = !(((1 << (range.end - range.start)) - 1) << range.start);

        *self &= mask;
        *self |= value << range.start;
    }

    #[inline(always)]
    fn set_bit(&mut self, bit: usize, value: bool) {
        *self &= !0 ^ (1 << bit);
        *self |= (value as Self) << bit;
    }
}

impl Bits for u64 {
    #[inline(always)]
    fn get_bits(&self, range: Range<usize>) -> u64 {
        let bits = *self << (64 - range.end) >> (64 - range.end);
        bits >> range.start
    }
    #[inline(always)]
    fn get_bit(&self, bit: usize) -> bool {
        (*self & 1 << bit) == 1 << bit
    }
    #[inline(always)]
    fn set_bits(&mut self, range: Range<usize>, value: Self) {
        let mask: Self = !(((1 << (range.end - range.start)) - 1) << range.start);
        *self &= mask;
        *self |= value << range.start;
    }
    #[inline(always)]
    fn set_bit(&mut self, bit: usize, value: bool) {
        *self &= !0 ^ (1 << bit);
        *self |= (value as Self) << bit;
    }
}

impl Bits for u16 {
    #[inline(always)]
    fn get_bits(&self, range: Range<usize>) -> u16 {
        let bits = *self << (16 - range.end) >> (16 - range.end);
        bits >> range.start
    }
    #[inline(always)]
    fn get_bit(&self, bit: usize) -> bool {
        (*self & 1 << bit) == 1 << bit
    }
    #[inline(always)]
    fn set_bits(&mut self, range: Range<usize>, value: Self) {
        let mask: Self = !(((1 << (range.end - range.start)) - 1) << range.start);

        *self &= mask;
        *self |= value << range.start;
    }
    #[inline(always)]
    fn set_bit(&mut self, bit: usize, value: bool) {
        *self &= !0 ^ (1 << bit);
        *self |= (value as Self) << bit;
    }
}

impl Bits for u8 {
    #[inline(always)]
    fn get_bits(&self, range: Range<usize>) -> u8 {
        let bits = *self << (8 - range.end) >> (8 - range.end);
        bits >> range.start
    }
    #[inline(always)]
    fn get_bit(&self, bit: usize) -> bool {
        (*self & 1 << bit) == 1 << bit
    }
    #[inline(always)]
    fn set_bits(&mut self, range: Range<usize>, value: Self) {
        let mask: Self = !(((1 << (range.end - range.start)) - 1) << range.start);

        *self &= mask;
        *self |= value << range.start;
    }
    #[inline(always)]
    fn set_bit(&mut self, bit: usize, value: bool) {
        *self &= !0 ^ (1 << bit);
        *self |= (value as Self) << bit;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_bits_u32_to_u32() {
        {
            // arrange
            let input: u32 = 0b0000_0000_0000_0000_0000_0000_1111_1111_u32;

            // act
            let o1: u32 = input.get_bits(0..8);
            let o2: u32 = input.get_bits(0..4);
            let o3 = input.get_bit(0);

            // assert
            assert_eq!(o1, 0b1111_1111_u32);
            assert_eq!(o2, 0b1111_u32);
            assert_eq!(o3, true);
        }
        {
            // arrange
            let input: u32 = 0b0000_0000_0000_0000_1100_0000_0000_0000_u32;

            // act
            let o1: u32 = input.get_bits(14..16);

            // assert
            assert_eq!(o1, 0b11_u32);
        }
        {
            // arrange
            let input: u32 = 0b1111_1111_1111_1111_1111_1111_1111_1111_u32;
            // act
            let o1: u32 = input.get_bits(0..32);

            // assert
            assert_eq!(o1, 0b1111_1111_1111_1111_1111_1111_1111_1111_u32);
        }
    }
}
