use std::{
    cmp::{max, min},
    ops::Index,
};

pub struct Slice<'a, T>(&'a [T], usize);
pub struct SliceIterator<'a, 'b, T>(&'b Slice<'a, T>, usize);

pub trait IntoSlice {
    type Item;

    fn slice(
        &self,
        index: usize,
        count: Option<usize>,
        step: Option<usize>,
    ) -> Slice<'_, Self::Item>;
}

impl<'a, T> Slice<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len() / self.1
    }
}

impl<'a, T> IntoSlice for &'a [T] {
    type Item = T;
    fn slice(
        &self,
        index: usize,
        count: Option<usize>,
        step: Option<usize>,
    ) -> Slice<'_, Self::Item> {
        let index = min(index, self.len());
        let step = max(step.unwrap_or(1), 1);
        let count = min(count.unwrap_or(self.len()), self.len() - index);
        Slice(&self[index..index + count], step)
    }
}

impl<'a, T> IntoSlice for Slice<'a, T> {
    type Item = T;
    fn slice(
        &self,
        index: usize,
        count: Option<usize>,
        step: Option<usize>,
    ) -> Slice<'_, Self::Item> {
        let index = min(index, self.0.len());
        let step = max(step.unwrap_or(1), 1);
        let count = min(
            count.unwrap_or(self.0.len()),
            (self.0.len() - index + step - 1) / step,
        );
        let index = index * self.1;
        Slice(&self.0[index..index + count], step * self.1)
    }
}

impl<'a, T> Index<usize> for Slice<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index * self.1]
    }
}

impl<'a, 'b, T> IntoIterator for &'b Slice<'a, T> {
    type Item = &'a T;
    type IntoIter = SliceIterator<'a, 'b, T>;

    fn into_iter(self) -> Self::IntoIter {
        SliceIterator(self, 0)
    }
}

impl<'a, 'b, T> Iterator for SliceIterator<'a, 'b, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.0 .0.get(self.1)?;
        self.1 += self.0 .1;
        Some(item)
    }
}

impl<'a, T> From<&'a [T]> for Slice<'a, T> {
    fn from(slice: &'a [T]) -> Self {
        Self(slice, 1)
    }
}
