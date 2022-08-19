use crate::{
    depend::Dependency,
    error::{Error, Result},
    identify,
    inject::{Adapt, Context, Get, Inject},
    resource::Write,
};
use entia_core::FullIterator;
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    iter::once,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Defer<'a, R: Resolve> {
    reserved: &'a AtomicUsize,
    indices: Option<&'a mut Vec<(usize, usize)>>,
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
    post: fn(&mut dyn Any) -> Result,
    resolve: fn(&mut dyn Any, usize) -> Result,
    depend: fn(&dyn Any) -> Vec<Dependency>,
}

struct Outer {
    indices: HashMap<usize, usize>,
    inners: Vec<Inner>,
}

struct Inner {
    identifier: usize,
    reserved: AtomicUsize,
    resolved: usize,
    indices: Vec<(usize, usize)>,
    resolvers: Vec<Resolver>,
}

pub trait Resolve {
    type Item;

    fn resolve(&mut self, items: impl FullIterator<Item = Self::Item>) -> Result;
    fn depend(&self) -> Vec<Dependency>;

    #[inline]
    fn pre(&mut self) -> Result {
        Ok(())
    }

    #[inline]
    fn post(&mut self) -> Result {
        Ok(())
    }
}

impl Default for Outer {
    fn default() -> Self {
        Outer {
            indices: HashMap::new(),
            inners: Vec::new(),
        }
    }
}

#[allow(type_alias_bounds)]
type Triple<R: Resolve> = (R, Vec<(usize, usize)>, VecDeque<R::Item>);

impl Resolver {
    pub fn new<R: Resolve + Send + Sync + 'static>(state: R) -> Self
    where
        R::Item: Send + Sync,
    {
        Self {
            state: Box::new((
                state,
                Vec::<(usize, usize)>::new(),
                VecDeque::<R::Item>::new(),
            )),
            post: |state| {
                let (state, _, _) = state.downcast_mut::<Triple<R>>().ok_or(Error::WrongState)?;
                state.post()
            },
            resolve: |state, count| {
                let (state, _, items) =
                    state.downcast_mut::<Triple<R>>().ok_or(Error::WrongState)?;
                state.resolve(items.drain(..count))
            },
            depend: |state| match state.downcast_ref::<Triple<R>>() {
                Some((state, _, _)) => state.depend(),
                None => vec![Dependency::Unknown],
            },
        }
    }

    #[inline]
    pub fn post(&mut self) -> Result {
        (self.post)(&mut self.state)
    }

    #[inline]
    pub fn resolve(&mut self, count: usize) -> Result {
        (self.resolve)(&mut self.state, count)
    }

    #[inline]
    pub fn depend(&self) -> Vec<Dependency> {
        (self.depend)(&self.state)
    }

    #[inline]
    pub fn state_ref<R: Resolve + 'static>(&self) -> Result<&Triple<R>> {
        self.state.downcast_ref().ok_or(Error::WrongState)
    }

    #[inline]
    pub fn state_mut<R: Resolve + 'static>(&mut self) -> Result<&mut Triple<R>> {
        self.state.downcast_mut().ok_or(Error::WrongState)
    }
}

impl<R: Resolve> Defer<'_, R> {
    #[inline]
    pub fn one(&mut self, item: R::Item) {
        self.items.push_back(item);
        if let Some(indices) = self.indices.as_mut() {
            let index = self.reserved.fetch_add(1, Ordering::Relaxed);
            indices.push((index, 1))
        };
    }

    #[inline]
    pub fn all(&mut self, items: impl IntoIterator<Item = R::Item>) {
        let start = self.items.len();
        self.items.extend(items);
        if let Some(indices) = self.indices.as_mut() {
            let count = self.items.len() - start;
            if count > 0 {
                let index = self.reserved.fetch_add(1, Ordering::Relaxed);
                indices.push((index, count))
            }
        }
    }
}

unsafe impl<R: Resolve + Send + Sync + 'static> Inject for Defer<'_, R>
where
    R::Item: Send + Sync + 'static,
{
    type Input = R;
    type State = State<R>;

    fn initialize<A: Adapt<Self::State>>(
        input: Self::Input,
        mut context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        let mut outer = Write::<Outer>::initialize(None, context.map(|state| &mut state.outer))?;
        let identifier = context.identifier();
        let inner = {
            match outer.indices.get(&identifier) {
                Some(&index) => index,
                None => {
                    let index = outer.inners.len();
                    outer.indices.insert(identifier, index);
                    outer.inners.push(Inner {
                        identifier: identify(),
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
            inner.resolvers.push(Resolver::new(input));
            index
        };

        context.schedule(|state, mut schedule| {
            let outer = &state.outer;
            let inner = &outer.inners[state.inner];
            // Accumulate the dependencies of previous resolvers (including self) because some of their items may be resolved by
            // this run. This assumes that 'schedule' is called in the same order as 'initialize'.
            let dependencies = inner.resolvers[..=state.resolver]
                .iter()
                .enumerate()
                // TODO: Combine these in a way that remove inner conflicts between resolvers (keep the stricter dependencies).
                .flat_map(|(index, resolver)| {
                    once(Dependency::write::<Inner>(inner.identifier).at(index))
                        .chain(resolver.depend())
                });
            schedule.post(
                |state| {
                    let inner = &mut state.outer.inners[state.inner];
                    let reserved = inner.reserved.get_mut();
                    let resolvers = inner.resolvers.len();
                    let (previous, current) = inner.resolvers.split_at_mut(state.resolver);
                    let (resolver, indices, items) = current[0].state_mut::<R>()?;

                    resolver.pre()?;
                    // This check is fine since the only way there could be pending items in `inner.indices` is if they were waiting on
                    // another item to be resolved, thus only a defer with `items.len() > 0` could be blocking and will be responsible
                    // for resolving the pending items. Having no `items` also means that `indices` is empty.
                    if items.len() == 0 {
                        resolver.post()?;
                        return Ok(());
                    } else if resolvers <= 1 {
                        resolver.resolve(items.drain(..))?;
                        resolver.post()?;
                        return Ok(());
                    }

                    let mut resolve = 0;
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
                        // Resolve the items of this 'Defer' instance if possible without going through the abstract 'Resolver'.
                        resolver.resolve(items.drain(..resolve))?;
                    }

                    while let Some((index, count)) = inner.indices.get_mut(inner.resolved) {
                        match previous.get_mut(*index) {
                            Some(resolver) => {
                                *index = usize::MAX;
                                inner.resolved += 1;
                                resolver.resolve(*count)?;
                            }
                            // Can't make further progress; other 'Defer' instances will need to complete the resolution.
                            None => return Ok(()),
                        }
                    }

                    // The only way to get here is if all deferred items have been properly resolved.
                    debug_assert_eq!(*reserved, inner.resolved);
                    for resolver in previous {
                        resolver.post()?;
                    }
                    resolver.post()?;

                    *reserved = 0;
                    inner.resolved = 0;
                    Ok(())
                },
                dependencies,
            );
        });

        Ok(State {
            outer,
            inner,
            resolver,
            _marker: PhantomData,
        })
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        let inner = &state.outer.inners[state.inner];
        vec![Dependency::write::<Inner>(inner.identifier).at(state.resolver)]
    }
}

impl<'a, R: Resolve + 'static> Get<'a> for State<R> {
    type Item = (Defer<'a, R>, &'a mut R);

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        let inner = &mut self.outer.inners[self.inner];
        let count = inner.resolvers.len();
        let resolver = &mut inner.resolvers[self.resolver];
        let (resolver, indices, items) = resolver.state_mut::<R>().unwrap();
        let indices = if count <= 1 { None } else { Some(indices) };
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
