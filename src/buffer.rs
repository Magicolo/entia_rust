use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};
use std::ptr;

pub struct Buffer<T> {
    count: usize,
    chunks: Vec<Box<[T; Buffer::<()>::CHUNK]>>,
}

pub struct BufferIterator<'a, T> {
    front: usize,
    back: usize,
    buffer: &'a Buffer<T>,
}

impl<T> Buffer<T> {
    pub const CHUNK: usize = 32;

    #[inline]
    pub const fn new() -> Self {
        Self {
            count: 0,
            chunks: Vec::new(),
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.count {
            let indices = Self::adjust(index);
            Some(&self.chunks[indices.0][indices.1])
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.count {
            let indices = Self::adjust(index);
            Some(&mut self.chunks[indices.0][indices.1])
        } else {
            None
        }
    }

    #[inline]
    pub fn push(&mut self, value: T) -> &mut T {
        let indices = Self::adjust(self.count);
        if indices.0 == 0 {
            unsafe {
                let chunk = Box::new(MaybeUninit::uninit().assume_init());
                self.chunks.push(chunk);
            }
        }
        self.count += 1;
        let slot = &mut self.chunks[indices.0][indices.1];
        *slot = value;
        slot
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }

        self.count -= 1;
        let indices = Self::adjust(self.count);
        unsafe { Some(ptr::read(self.chunks[indices.0].as_ptr().add(indices.1))) }
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.chunks.clear();
    }

    #[inline]
    const fn adjust(index: usize) -> (usize, usize) {
        (index / Self::CHUNK, index % Self::CHUNK)
    }
}

impl<T> Index<usize> for Buffer<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        let indices = Self::adjust(index);
        &self.chunks[indices.0][indices.1]
    }
}

impl<T> IndexMut<usize> for Buffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let indices = Self::adjust(index);
        &mut self.chunks[indices.0][indices.1]
    }
}

impl<'a, T> Iterator for BufferIterator<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.nth(0)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if self.len() > n {
            let index = self.front + n;
            self.front = index + 1;
            Some(&self.buffer[index])
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.back - self.front;
        (count, Some(count))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl<T> ExactSizeIterator for BufferIterator<'_, T> {}

impl<T> DoubleEndedIterator for BufferIterator<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.nth_back(0)
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        if self.len() > n {
            let index = self.back - n - 1;
            self.back = index;
            Some(&self.buffer[index])
        } else {
            None
        }
    }
}

impl<T> FusedIterator for BufferIterator<'_, T> {}

impl<'a, T> IntoIterator for &'a Buffer<T> {
    type Item = &'a T;
    type IntoIter = BufferIterator<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            front: 0,
            back: self.count,
            buffer: self,
        }
    }
}
