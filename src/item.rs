use crate::{depend::Depend, inject::InjectContext, segment::Segment, world::World};

pub struct ItemContext<'a> {
    identifier: usize,
    segment: usize,
    world: &'a mut World,
}

pub unsafe trait Item: Send {
    type State: for<'a> At<'a> + Depend + Send + 'static;
    fn initialize(context: ItemContext) -> Option<Self::State>;
    #[inline]
    fn update(_: &mut Self::State, _: ItemContext) {}
}

pub trait At<'a> {
    type Item;
    fn at(&'a self, index: usize, world: &'a World) -> Self::Item;
}

impl<'a> ItemContext<'a> {
    pub fn new(identifier: usize, segment: usize, world: &'a mut World) -> Self {
        Self {
            identifier,
            segment,
            world,
        }
    }

    pub fn identifier(&self) -> usize {
        self.identifier
    }

    pub fn segment(&self) -> &Segment {
        &self.world.segments[self.segment]
    }

    pub fn world(&mut self) -> &mut World {
        self.world
    }

    pub fn owned(&mut self) -> ItemContext {
        self.with(self.segment)
    }

    pub fn with(&mut self, segment: usize) -> ItemContext {
        ItemContext::new(self.identifier, segment, self.world)
    }
}

impl<'a> Into<InjectContext<'a>> for ItemContext<'a> {
    fn into(self) -> InjectContext<'a> {
        InjectContext::new(self.identifier, self.world)
    }
}

unsafe impl<I: Item> Item for Option<I> {
    type State = Option<I::State>;

    fn initialize(context: ItemContext) -> Option<Self::State> {
        Some(I::initialize(context))
    }

    fn update(state: &mut Self::State, context: ItemContext) {
        if let Some(state) = state {
            I::update(state, context);
        }
    }
}

impl<'a, A: At<'a>> At<'a> for Option<A> {
    type Item = Option<A::Item>;

    #[inline]
    fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
        Some(self.as_ref()?.at(index, world))
    }
}

macro_rules! item {
    ($($p:ident, $t:ident),*) => {
        unsafe impl<$($t: Item,)*> Item for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(mut _context: ItemContext) -> Option<Self::State> {
                Some(($($t::initialize(_context.owned())?,)*))
            }

            fn update(($($p,)*): &mut Self::State, mut _context: ItemContext) {
                $($t::update($p, _context.owned());)*
            }
        }

        impl<'a, $($t: At<'a>,)*> At<'a> for ($($t,)*) {
            type Item = ($($t::Item,)*);

            #[inline]
            fn at(&'a self, _index: usize, _world: &'a World) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.at(_index, _world),)*)
            }
        }
    };
}

entia_macro::recurse_32!(item);
