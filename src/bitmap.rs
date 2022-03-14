const BITMAP_BLOCK_SIZE: usize = 1 << 3;
const BITMAP_BLOCK_MASK: usize = BITMAP_BLOCK_SIZE - 1;

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

pub fn all(vec: &[u8]) -> bool {
    vec.iter().all(|&x| x == 0xff)
}

pub fn any(vec: &[u8]) -> bool {
    !none(vec)
}

pub fn none(vec: &[u8]) -> bool {
    vec.iter().all(|&x| x == 0x00)
}

pub fn set_all(vec: &mut [u8]) {
    vec.fill(0xff)
}

pub fn clear_all(vec: &mut [u8]) {
    vec.fill(0)
}

pub fn set_bit(vec: &mut [u8], n: usize) {
    vec[n / BITMAP_BLOCK_SIZE] |= 1 << (n & BITMAP_BLOCK_MASK);
}

pub fn clear_bit(vec: &mut [u8], n: usize) {
    vec[n / BITMAP_BLOCK_SIZE] &= !(1 << (n & BITMAP_BLOCK_MASK));
}
