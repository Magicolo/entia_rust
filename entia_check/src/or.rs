use Or::*;

use crate::{
    generator::{Generate, State},
    shrink::Shrink,
};

#[derive(Clone, Debug)]
pub enum Or<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Or<Or<L, R>, Or<L, R>> {
    #[inline]
    pub fn flatten(self) -> Or<L, R> {
        match self {
            Left(Left(left)) => Left(left),
            Left(Right(right)) => Right(right),
            Right(Left(left)) => Left(left),
            Right(Right(right)) => Right(right),
        }
    }
}

impl<L, R> Or<Or<L, R>, R> {
    #[inline]
    pub fn flatten_left(self) -> Or<L, R> {
        match self {
            Left(Left(left)) => Left(left),
            Left(Right(right)) => Right(right),
            Right(right) => Right(right),
        }
    }
}

impl<L, R> Or<L, Or<L, R>> {
    #[inline]
    pub fn flatten_right(self) -> Or<L, R> {
        match self {
            Left(left) => Left(left),
            Right(Left(left)) => Left(left),
            Right(Right(right)) => Right(right),
        }
    }
}

impl<L: Generate, R: Generate<Item = L::Item>> Generate for Or<L, R> {
    type Item = L::Item;
    type Shrink = Or<L::Shrink, R::Shrink>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        match self {
            Left(generate) => {
                let (item, shrink) = generate.generate(state);
                (item, Left(shrink))
            }
            Right(generate) => {
                let (item, shrink) = generate.generate(state);
                (item, Right(shrink))
            }
        }
    }
}

impl<L: Shrink, R: Shrink<Item = L::Item>> Shrink for Or<L, R> {
    type Item = L::Item;

    fn generate(&self) -> Self::Item {
        match self {
            Left(left) => left.generate(),
            Right(right) => right.generate(),
        }
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(match self {
            Left(left) => Left(left.shrink()?),
            Right(right) => Right(right.shrink()?),
        })
    }
}
