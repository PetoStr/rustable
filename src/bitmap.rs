//! Implementation of bitmap operations on the slice of bytes.

const BITMAP_BLOCK_SIZE: usize = 1 << 3;
const BITMAP_BLOCK_MASK: usize = BITMAP_BLOCK_SIZE - 1;

/// Performs logical and, modifying the `left` argument. If the sizes do not match, additional
/// bytes are treated as ones.
///
/// Returns an exclusive reference to `left`.
pub fn and<'a, 'b>(left: &'a mut [u8], right: &'b [u8]) -> &'a mut [u8] {
    let len = left.len().min(right.len());

    // optimize bounds checking
    let left = &mut left[..len];
    let right = &right[..len];

    for i in 0..len {
        left[i] &= right[i];
    }

    left
}

/// Returns `true` if all bits are 1.
pub fn all(vec: &[u8]) -> bool {
    vec.iter().all(|&x| x == 0xff)
}

/// Returns `true` if any bit is 1.
pub fn any(vec: &[u8]) -> bool {
    !none(vec)
}

/// Returns `true` if all bits are 0.
pub fn none(vec: &[u8]) -> bool {
    vec.iter().all(|&x| x == 0x00)
}

/// Sets all bits to 1.
pub fn set_all(vec: &mut [u8]) {
    vec.fill(0xff)
}

/// Sets all bits to 0.
pub fn clear_all(vec: &mut [u8]) {
    vec.fill(0)
}

/// Sets bit at an index `n`.
pub fn set_bit(vec: &mut [u8], n: usize) {
    vec[n / BITMAP_BLOCK_SIZE] |= 1 << (n & BITMAP_BLOCK_MASK);
}

/// Clears bit at an index `n`.
pub fn clear_bit(vec: &mut [u8], n: usize) {
    vec[n / BITMAP_BLOCK_SIZE] &= !(1 << (n & BITMAP_BLOCK_MASK));
}
