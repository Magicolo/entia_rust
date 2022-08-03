use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{Context, Get, Inject},
    resource::Write,
    world::World,
};
use entia_core::FullIterator;
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    marker::PhantomData,
    mem::replace,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Defer<'a, R: Resolve> {
    reserved: &'a AtomicUsize,
    indices: &'a mut Vec<(usize, usize)>,
    items: &'a mut VecDeque<R::Item>,
}

pub struct State<T> {
    outer: Write<Outer>,
    inner: usize,
    resolver: usize,
    _marker: PhantomData<T>,
}

struct Resolver {
    state: Box<dyn Any + Send + Sync>,
    resolve: fn(usize, &mut dyn Any, &mut World) -> Result,
}

struct Outer {
    indices: HashMap<usize, usize>,
    inners: Vec<Inner>,
}

struct Inner {
    reserved: AtomicUsize,
    resolved: usize,
    indices: Vec<(usize, usize)>,
    resolvers: Vec<Resolver>,
}

#[allow(type_alias_bounds)]
type Triple<R: Resolve> = (R, Vec<(usize, usize)>, VecDeque<R::Item>);

/*
    TODO:
    Is is possible to share the 'reserved' counter between systems such that no identifier is needed for systems?
    - There must be a strict ordering between the overlaping operations of system A and system B.
    - Currently all deferred operations (including 'Create/Destroy/Adopt/Reject/Add/Remove') depend on 'Write<Entities>',
    which means that they can't be resolved concurrently.
    - 'Create' can be more specific by depending on:
        inject: for each segment: Read<Entities>.at(segment) + Read<Segment>.at(segment)
        resolve: for each segment: Write<Entities>.at(segment) + Write<Segment>.at(segment)
        - This way, it doesn't overlap with 'Query' if the segments don't overlap.
            inject: for each segment: Read<Entities>.at(segment) /* because of query.get */ + Read<Segment>.at(segment)
    - 'Adopt/Reject' can be more specific by depending on:
        inject: Nothing?
        resolve: Read<Entities> + Write<Datum> // since it only changes the family members
        - This way, there is a way for 'Query' to not overlap if it doesn't read the family members.
*/

pub trait Resolve {
    type Item;
    fn resolve(&mut self, items: impl FullIterator<Item = Self::Item>, world: &mut World)
        -> Result;
}

impl Default for Outer {
    fn default() -> Self {
        Outer {
            indices: HashMap::new(),
            inners: Vec::new(),
        }
    }
}

impl Resolver {
    #[inline]
    pub fn resolve(&mut self, count: usize, world: &mut World) -> Result {
        (self.resolve)(count, self.state.as_mut(), world)
    }

    #[inline]
    pub fn state_ref<R: Resolve + 'static>(&self) -> Option<&Triple<R>> {
        self.state.downcast_ref()
    }

    #[inline]
    pub fn state_mut<R: Resolve + 'static>(&mut self) -> Option<&mut Triple<R>> {
        self.state.downcast_mut()
    }
}

impl<R: Resolve> Defer<'_, R> {
    #[inline]
    pub fn one(&mut self, item: R::Item) {
        let index = self.reserved.fetch_add(1, Ordering::Relaxed);
        self.items.push_back(item);
        self.indices.push((index, 1));
    }

    #[inline]
    pub fn all(&mut self, items: impl IntoIterator<Item = R::Item>) {
        let index = self.reserved.fetch_add(1, Ordering::Relaxed);
        let start = self.items.len();
        self.items.extend(items);
        let count = self.items.len() - start;
        self.indices.push((index, count));
    }
}

impl<R: Resolve + Send + Sync + 'static> Inject for Defer<'_, R>
where
    <R as Resolve>::Item: Send + Sync + 'static,
{
    type Input = R;
    type State = State<R>;

    fn initialize(input: Self::Input, context: Context) -> Result<Self::State> {
        let identifier = context.identifier();
        let mut outer = <Write<Outer> as Inject>::initialize(None, context)?;
        let inner = {
            match outer.indices.get(&identifier) {
                Some(&index) => index,
                None => {
                    let index = outer.inners.len();
                    outer.indices.insert(identifier, index);
                    outer.inners.push(Inner {
                        reserved: AtomicUsize::new(0),
                        resolved: 0,
                        indices: Vec::new(),
                        resolvers: Vec::new(),
                    });
                    index
                }
            }
        };

        let resolver = {
            let inner = &mut outer.inners[inner];
            let index = inner.resolvers.len();
            inner.resolvers.push(Resolver {
                state: Box::new((
                    input,
                    Vec::<(usize, usize)>::new(),
                    VecDeque::<R::Item>::new(),
                )),
                resolve: |count, state, world| {
                    let (state, _, items) = state
                        .downcast_mut::<Triple<R>>()
                        .expect("Invalid resolve state.");
                    state.resolve(items.drain(..count), world)
                },
            });
            index
        };

        Ok(State {
            outer,
            inner,
            resolver,
            _marker: PhantomData,
        })
    }

    fn resolve(state: &mut Self::State, mut context: Context) -> Result {
        let inner = &mut state.outer.inners[state.inner];
        let mut resolve = 0;
        let reserved = inner.reserved.get_mut();
        let resolver = &mut inner.resolvers[state.resolver];
        let (resolver, indices, items) = resolver.state_mut::<R>().unwrap();
        inner.indices.resize(*reserved, (usize::MAX, 0));

        for (index, count) in indices.drain(..) {
            if index == inner.resolved {
                inner.resolved += 1;
                resolve += count;
            } else {
                inner.indices[index] = (state.resolver, count);
            }
        }

        if resolve > 0 {
            // Resolve the items of this 'Defer' instance if possible without the to go through the abstract 'Resolver'.
            resolver.resolve(items.drain(..resolve), context.world())?;
        }

        while let Some((resolver, count)) = inner.indices.get_mut(inner.resolved) {
            match inner.resolvers.get_mut(replace(resolver, usize::MAX)) {
                Some(resolver) => {
                    inner.resolved += 1;
                    resolver.resolve(*count, context.world())?;
                }
                // Can't make further progress; other 'Defer' instances will need to complete the resolution.
                None => return Ok(()),
            }
        }

        // The only way to get here is if all deferred items have been properly resolved.
        debug_assert_eq!(*reserved, inner.resolved);
        *reserved = 0;
        inner.resolved = 0;
        Ok(())
    }
}

impl<'a, R: Resolve + 'static> Get<'a> for State<R> {
    type Item = (Defer<'a, R>, &'a mut R);

    #[inline]
    unsafe fn get(&'a mut self, _: &World) -> Self::Item {
        let inner = &mut self.outer.inners[self.inner];
        let resolver = &mut inner.resolvers[self.resolver];
        let (resolver, indices, items) = resolver.state_mut::<R>().unwrap();
        (
            Defer {
                reserved: &inner.reserved,
                indices,
                items,
            },
            resolver,
        )
    }
}

unsafe impl<T> Depend for State<T> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        self.outer.depend(world)
    }
}

impl<R: Resolve + 'static> AsRef<R> for State<R> {
    fn as_ref(&self) -> &R {
        let resolver = &self.outer.inners[self.inner].resolvers[self.resolver];
        &resolver.state_ref::<R>().unwrap().0
    }
}

impl<R: Resolve + 'static> AsMut<R> for State<R> {
    fn as_mut(&mut self) -> &mut R {
        let resolver = &mut self.outer.inners[self.inner].resolvers[self.resolver];
        &mut resolver.state_mut::<R>().unwrap().0
    }
}
