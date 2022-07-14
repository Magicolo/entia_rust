use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    destroy::{Early, Late},
    entities::{Datum, Entities},
    entity::Entity,
    error::{Error, Result},
    family::template::{EntityIndices, SegmentIndices},
    inject::Inject,
    item::{At, Context, Item},
    resource,
    resource::Write,
    segment::Segment,
    template::{
        ApplyContext, CountContext, DeclareContext, InitializeContext, LeafTemplate, Template,
    },
    world::World,
};
use std::{collections::HashMap, marker::PhantomData};

/*
    |temperature: &Temperature, time: &Time, query: Query<(&mut Cold, Add<_, Early|Late>)>| {
        for (cold, mut add) in query.iter() {
            cold.0 = cold.0.lerp(temperature.celcius, time.delta).min(0);
            if cold.0 < -15 {
                add.one(Frozen);
            }
        }
    }
    // For each segment of the query, try to create a segment with the added component.
*/
// todo!();
pub struct Add<'a, T: LeafTemplate + 'a, R = Early> {
    defer: defer::Defer<'a, Outer<T>>,
    inner: &'a mut Inner<T>,
    entities: &'a Entities,
    world: &'a World,
    _marker: PhantomData<R>,
}
pub struct State<T: Template, R>(defer::State<Outer<T>>, PhantomData<R>);

struct Outer<T: Template> {
    inner: Inner<T>,
    entities: Write<Entities>,
}

struct Inner<T: Template> {
    segments: (usize, usize),
    segment_indices: SegmentIndices,
    entity_indices: EntityIndices,
    initial_state: <T as Template>::State,
    initialize: Vec<(u32, Datum)>,
}

struct Defer<T> {
    initial_root: T,
    entity_instance: Entity,
    entity_indices: EntityIndices,
    segment_indices: SegmentIndices,
}

impl<T: LeafTemplate> Add<'_, T, Early> {
    pub fn one(&mut self, template: T) {
        // self.defer.one();
    }
}

impl<T: LeafTemplate> Add<'_, T, Late> {
    pub fn one(&mut self, template: T) {
        // self.defer.one();
    }
}

impl<T: LeafTemplate + Send + Sync + 'static, R: 'static> Item for Add<'_, T, R>
where
    State<T, R>: Depend,
{
    type State = State<T, R>;

    fn initialize(mut context: Context) -> Result<Self::State> {
        let entities =
            <resource::Write<Entities> as Inject>::initialize(None, context.owned().into())?;
        let mut metas = vec![vec![]];
        let initial = T::declare(DeclareContext::new(0, &mut metas, context.world()));
        let types = metas[0]
            .iter()
            .map(|meta| meta.identifier())
            .chain(context.segment().component_types().iter().copied())
            .collect::<Vec<_>>();
        let segment = context.world().get_or_add_segment_with(types).index();
        let metas_to_segment = [(0, 0)].into_iter().collect::<HashMap<_, _>>();
        let mut segment_indices = vec![SegmentIndices {
            segment,
            count: 0,
            index: 0,
            store: 0,
        }];
        let state = T::initialize(
            initial,
            InitializeContext::new(0, &segment_indices, &metas_to_segment, context.world()),
        );

        let mut entity_indices = Vec::new();
        if T::static_count(
            &state,
            CountContext::new(&mut segment_indices, &mut None, &mut entity_indices),
        )? {
        } else {
            return Err(Error::StaticCountMustBeTrue);
        }

        ApplyContext::new(
            (0, 0),
            &[],
            &[EntityIndices {
                segment,
                offset: 0,
                parent: None,
                next_sibling: None,
                previous_sibling: None,
            }],
            &segment_indices,
            &mut vec![],
        );
        context.segment().entity_store();
        let defer = <defer::Defer<_> as Inject>::initialize(
            Outer {
                inner: Inner {
                    segments: (context.segment().index(), segment),
                    segment_indices: segment_indices
                        .into_iter()
                        .next()
                        .expect("Expected segment indices."),
                    entity_indices: entity_indices
                        .into_iter()
                        .next()
                        .expect("Expected entity indices."),
                    initial_state: state,
                    initialize: Vec::new(),
                },
                entities,
            },
            context.into(),
        )?;
        Ok(State(defer, PhantomData))
    }
}

pub struct AddChunk<'a, T, R>(PhantomData<&'a ()>, PhantomData<T>, PhantomData<R>);
impl<'a, T: LeafTemplate + 'static, R: 'static> At<'a, usize> for State<T, R> {
    type State = ();
    type Ref = ();
    type Mut = ();

    fn get(&'a self, segment: &Segment) -> Option<Self::State> {
        todo!()
    }

    unsafe fn at_ref(state: &Self::State, index: usize) -> Self::Ref {
        todo!()
    }

    unsafe fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
        todo!()
    }
}

// impl<T: LeafTemplate, R> At<usize> for AddChunk<'_, T, R> {
//     type Ref<'a> = Add<'a, T, R> where Self: 'a;
//     type Mut<'a> = Self::Ref<'a> where Self: 'a;

//     fn at<'a>(&'a self, index: usize) -> Option<Self::Ref<'a>> {
//         todo!()
//     }

//     unsafe fn at_unchecked<'a>(&'a self, index: usize) -> Self::Ref<'a> {
//         todo!()
//     }

//     fn at_mut<'a>(&'a mut self, index: usize) -> Option<Self::Mut<'a>> {
//         todo!()
//     }

//     unsafe fn at_unchecked_mut<'a>(&'a mut self, index: usize) -> Self::Mut<'a> {
//         todo!()
//     }
// }

impl<T: Template> Resolve for Outer<T> {
    type Item = Defer<T>;

    fn resolve(
        &mut self,
        items: impl ExactSizeIterator<Item = Self::Item>,
        world: &mut World,
    ) -> Result {
        let entities = self.entities.as_mut();
        todo!()
    }
}

unsafe impl<T: Template> Depend for State<T, Early> {
    fn depend(&self, world: &crate::World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        // TODO: Depend on segments
        dependencies
    }
}

unsafe impl<T: Template> Depend for State<T, Late> {
    fn depend(&self, world: &crate::World) -> Vec<Dependency> {
        self.0.depend(world)
    }
}
