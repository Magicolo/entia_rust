use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
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
    pub(crate) const fn new(entity: Entity, entities: &'a Entities) -> Self {
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
    pub fn children(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .children(entity)
            .map(move |child| Self(child, entities))
    }

    #[inline]
    pub fn ancestors(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .ancestors(entity)
            .map(move |parent| Self(parent, entities))
    }

    #[inline]
    pub fn descendants(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .descendants(entity)
            .map(move |child| Self(child, entities))
    }

    #[inline]
    pub fn siblings(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .siblings(entity)
            .map(move |sibling| Self(sibling, entities))
    }

    #[inline]
    pub fn ascend(
        &self,
        mut up: impl FnMut(Self) -> bool,
        mut down: impl FnMut(Self) -> bool,
    ) -> Option<bool> {
        self.1.ascend(
            self.0,
            |parent| up(Self(parent, self.1)),
            |parent| down(Self(parent, self.1)),
        )
    }

    #[inline]
    pub fn descend(
        &self,
        mut down: impl FnMut(Self) -> bool,
        mut up: impl FnMut(Self) -> bool,
    ) -> Option<bool> {
        self.1.descend(
            self.0,
            |child| down(Self(child, self.1)),
            |child| up(Self(child, self.1)),
        )
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
            .field("parent", &self.parent().map(|parent| parent.entity()))
            .field(
                "children",
                &self
                    .children()
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
