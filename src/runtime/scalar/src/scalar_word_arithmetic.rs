use super::finite_power::{NormalArithmeticError, RoundedBinary64, round_u128_binary64_unbounded};
use super::shared_scalar_f32::ScalarF32Rounding;

pub(super) fn shifted_words<const N: usize>(value: u128, shift: u32)
    -> Result<[u64; N], NormalArithmeticError> {
    let width = 128 - value.leading_zeros();
    if shift + width > (N as u32) * 64 { return Err(NormalArithmeticError::AlignmentOutsideDomain); }
    let mut words = [0u64; N];
    let offset = shift % 64;
    let first = (shift / 64) as usize;
    for (index, limb) in [value as u64, (value >> 64) as u64].iter().copied().enumerate() {
        if limb == 0 { continue; }
        words[first + index] |= limb << offset;
        if offset != 0 && first + index + 1 < N {
            words[first + index + 1] |= limb >> (64 - offset);
        }
    }
    Ok(words)
}

pub(super) fn combine_words<const N: usize>(left: [u64; N], left_negative: bool,
    right: [u64; N], right_negative: bool) -> Result<([u64; N], bool), NormalArithmeticError> {
    let mut exact = [0u64; N];
    let negative;
    if left_negative == right_negative {
        let mut carry = false;
        for index in 0..N {
            let (partial, first) = left[index].overflowing_add(right[index]);
            let (sum, second) = partial.overflowing_add(u64::from(carry));
            exact[index] = sum;
            carry = first || second;
        }
        if carry { return Err(NormalArithmeticError::AlignmentOutsideDomain); }
        negative = left_negative;
    } else {
        let ordering = left.iter().rev().cmp(right.iter().rev());
        if ordering == std::cmp::Ordering::Equal { return Ok((exact, false)); }
        let (large, small) = if ordering == std::cmp::Ordering::Greater {
            negative = left_negative;
            (left, right)
        } else {
            negative = right_negative;
            (right, left)
        };
        let mut borrow = false;
        for index in 0..N {
            let (partial, first) = large[index].overflowing_sub(small[index]);
            let (difference, second) = partial.overflowing_sub(u64::from(borrow));
            exact[index] = difference;
            borrow = first || second;
        }
    }
    Ok((exact, negative))
}

fn any_low_words<const N: usize>(words: [u64; N], count: u32) -> bool {
    let whole = (count / 64) as usize;
    let part = count % 64;
    words[..whole].iter().any(|word| *word != 0)
        || (part != 0 && words[whole] & ((1u64 << part) - 1) != 0)
}

pub(super) fn round_words_unbounded<const N: usize>(words: [u64; N], power: i32,
    negative: bool, rounding: ScalarF32Rounding) -> Option<RoundedBinary64> {
    let high_word = words.iter().rposition(|word| *word != 0)?;
    let highest = high_word as u32 * 64 + 63 - words[high_word].leading_zeros();
    let discard = highest.saturating_sub(55);
    let word = (discard / 64) as usize;
    let offset = discard % 64;
    let mut head = words[word] >> offset;
    if offset != 0 && word + 1 < N { head |= words[word + 1] << (64 - offset); }
    let jammed = head | u64::from(any_low_words(words, discard));
    Some(round_u128_binary64_unbounded(u128::from(jammed), power + discard as i32, negative, rounding))
}
