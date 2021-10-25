use crate::{
    depend::{Depend, Dependency},
    entities::{Entities, Horizontal, Vertical},
    entity::Entity,
    inject::Inject,
    query::item::{At, Item, ItemContext},
    read::{self, Read},
    world::World,
};
use std::fmt;

#[derive(Clone)]
pub struct Family<'a>(Entity, &'a Entities);
pub struct State(read::State<Entity>, read::State<Entities>);

impl<'a> Family<'a> {
    #[inline]
    pub const fn new(entity: Entity, entities: &'a Entities) -> Self {
        Self(entity, entities)
    }

    #[inline]
    pub const fn entity(&self) -> Entity {
        self.0
    }

    #[inline]
    pub fn root(&self) -> Self {
        Self(self.1.root(self.0), self.1)
    }

    #[inline]
    pub fn parent(&self) -> Option<Self> {
        Some(Self(self.1.parent(self.0)?, self.1))
    }

    #[inline]
    pub fn children(&self, direction: Horizontal) -> impl Iterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .children(entity, direction)
            .map(move |child| Self(child, entities))
    }

    #[inline]
    pub fn ancestors(&self, direction: Vertical) -> impl Iterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .ancestors(entity, direction)
            .map(move |parent| Self(parent, entities))
    }

    #[inline]
    pub fn descendants(
        &self,
        direction: (Horizontal, Vertical),
    ) -> impl Iterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .descendants(entity, direction)
            .map(move |child| Self(child, entities))
    }

    #[inline]
    pub fn siblings(&self, direction: Horizontal) -> impl Iterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .siblings(entity, direction)
            .map(move |sibling| Self(sibling, entities))
    }

    /// Parameter 'each' takes the current ancestor and returns a 'bool' that indicates if the ascension should continue.
    /// Return value will be 'true' only if all ancestors have been visited.
    #[inline]
    pub fn ascend(&self, direction: Vertical, mut each: impl FnMut(Self) -> bool) -> bool {
        self.1
            .ascend(self.0, direction, |parent| each(Self(parent, self.1)))
    }

    /// Parameter 'each' takes the current descendant and returns a 'bool' that indicates if the descent should continue.
    /// Return value will be 'true' only if all descendants have been visited.
    #[inline]
    pub fn descend(
        &self,
        direction: (Horizontal, Vertical),
        mut each: impl FnMut(Self) -> bool,
    ) -> bool {
        self.1
            .descend(self.0, direction, |child| each(Self(child, self.1)))
    }
}

impl Into<Entity> for Family<'_> {
    fn into(self) -> Entity {
        self.entity()
    }
}

impl fmt::Debug for Family<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("{:?}", self.entity()))
            .field("Parent", &self.parent().map(|parent| parent.entity()))
            .field(
                "Children",
                &self
                    .children(Horizontal::FromLeft)
                    .map(|child| child.entity())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

unsafe impl Item for Family<'_> {
    type State = State;

    fn initialize(mut context: ItemContext) -> Option<Self::State> {
        Some(State(
            <Read<Entity> as Item>::initialize(context.owned())?,
            <Read<Entities> as Inject>::initialize(None, context.into())?,
        ))
    }
}

impl<'a> At<'a> for State {
    type Item = Family<'a>;

    fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
        Family(*self.0.at(index, world), self.1.as_ref())
    }
}

unsafe impl Depend for State {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        // 'Family' may read entities from any segment.
        let mut dependencies = self.0.depend(world);
        dependencies.append(&mut self.1.depend(world));
        dependencies
    }
}
