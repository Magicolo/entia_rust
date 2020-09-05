use crate::component::Segment;
use crate::entity::Data;
use std::cell::UnsafeCell;
use std::sync::Arc;

pub(crate) struct Inner {
    pub data: Vec<Data>,
    pub free_indices: Vec<u32>,
    pub frozen_indices: Vec<u32>,
    pub segments: Vec<Segment>,
}

// TODO: offer a safe API at the level of the 'World' that will use a 'Mutex' (or 'RWLock'?) to operate safely
// on the unsafe API (represented by 'Inner')
#[derive(Clone)]
pub struct World {
    pub(crate) inner: Arc<UnsafeCell<Inner>>,
}

impl World {
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(UnsafeCell::new(Inner {
                data: Vec::new(),
                free_indices: Vec::new(),
                frozen_indices: Vec::new(),
                segments: Vec::new(),
            })),
        }
    }

    #[inline]
    pub(crate) unsafe fn get(&self) -> &mut Inner {
        &mut *self.inner.get()
    }
}

pub enum Node {
    Lazy(Box<dyn Fn(&World) -> Node>),
    Schedule(Box<dyn Fn(&World) -> Runner>),
    Resolve,
    If(Box<dyn Fn() -> bool>),
    Map(Box<dyn Fn(Runner) -> Runner>),
    Name(String),
    Sequence(Vec<Node>),
    Parallel(Vec<Node>),
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        match self {
            Node::Lazy(_)
            | Node::Schedule(_)
            | Node::Resolve
            | Node::If(_)
            | Node::Map(_)
            | Node::Name(_) => true,
            Node::Sequence(_) | Node::Parallel(_) => false,
        }
    }

    pub fn is_branch(&self) -> bool {
        !self.is_leaf()
    }

    pub fn expand(self, world: &World) -> Node {
        self.descend(&|node| {
            if let Node::Lazy(provide) = node {
                (*provide)(world)
            } else {
                node
            }
        })
    }

    pub fn descend<F: Fn(Node) -> Node>(self, replace: &F) -> Node {
        replace(match self {
            Node::Sequence(mut children) => Node::Sequence(
                children
                    .drain(..)
                    .map(|child| child.descend(replace))
                    .collect(),
            ),
            Node::Parallel(mut children) => Node::Parallel(
                children
                    .drain(..)
                    .map(|child| child.descend(replace))
                    .collect(),
            ),
            _ => self,
        })
    }
}

pub struct Runner {
    // run: Box<dyn Any>,
}
