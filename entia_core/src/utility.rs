#[inline]
pub fn get_mut2<T>(slice: &mut [T], indices: (usize, usize)) -> Option<(&mut T, &mut T)> {
    if indices.0 == indices.1 || indices.0 >= slice.len() || indices.1 >= slice.len() {
        let pointer = slice.as_mut_ptr();
        Some(unsafe { (&mut *pointer.add(indices.0), &mut *pointer.add(indices.1)) })
    } else {
        None
    }
}

pub fn next_power_of_2(mut value: u32) -> u32 {
    value |= value >> 1;
    value |= value >> 2;
    value |= value >> 4;
    value |= value >> 8;
    value |= value >> 16;
    value + 1
}
