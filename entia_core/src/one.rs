use std::convert::TryInto;

pub trait One<T> {
    fn one(self) -> Option<T>;
}

impl<T> One<T> for Box<[T]> {
    #[inline]
    fn one(self) -> Option<T> {
        match TryInto::<Box<[T; 1]>>::try_into(self) {
            Ok(array) => {
                let array = *array;
                Some(unsafe { array.as_ptr().read() })
            }
            Err(_) => None,
        }
    }
}
