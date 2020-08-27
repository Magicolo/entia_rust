use crate::component::Segment;
use crate::entity::Data;

#[derive(Default)]
pub struct World {
    pub(crate) data: Vec<Data>,
    pub(crate) free_indices: Vec<u32>,
    pub(crate) frozen_indices: Vec<u32>,
    pub(crate) segments: Vec<Segment>,
}

impl World {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resolve(&mut self) {
        self.free_indices.append(&mut self.frozen_indices);
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
