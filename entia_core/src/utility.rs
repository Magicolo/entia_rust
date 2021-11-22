use std::{
    any::type_name,
    mem::{forget, MaybeUninit},
};

pub fn array<T, const N: usize>(mut provide: impl FnMut(usize) -> T) -> [T; N] {
    struct Array<T>(*mut T, usize);
    impl<T> Drop for Array<T> {
        fn drop(&mut self) {
            for i in 0..self.1 {
                unsafe { self.0.add(i).read() };
            }
        }
    }

    let mut items: MaybeUninit<[T; N]> = MaybeUninit::uninit();
    let mut array = Array(items.as_mut_ptr().cast::<T>(), 0);

    for i in 0..N {
        let item = provide(i);
        unsafe { array.0.add(i).write(item) };
        array.1 += 1;
    }

    forget(array);
    unsafe { items.assume_init() }
}

pub fn get_mut2<T>(slice: &mut [T], indices: (usize, usize)) -> Option<(&mut T, &mut T)> {
    if indices.0 == indices.1 || indices.0 >= slice.len() || indices.1 >= slice.len() {
        None
    } else {
        let pointer = slice.as_mut_ptr();
        Some(unsafe { (&mut *pointer.add(indices.0), &mut *pointer.add(indices.1)) })
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

pub fn short_type_name<T: ?Sized>() -> String {
    let name = type_name::<T>();
    let mut buffer = String::with_capacity(name.len());
    let mut checkpoint = 0;
    let mut characters = name.chars();

    while let Some(character) = characters.next() {
        if character == ':' {
            match characters.next() {
                Some(':') => buffer.truncate(checkpoint),
                Some(character) => {
                    buffer.push(':');
                    buffer.push(character);
                    checkpoint = buffer.len();
                }
                None => {
                    buffer.push(':');
                    checkpoint = buffer.len();
                }
            }
        } else if character == '_' || character.is_alphanumeric() {
            buffer.push(character);
        } else {
            buffer.push(character);
            checkpoint = buffer.len();
        }
    }
    buffer
}
