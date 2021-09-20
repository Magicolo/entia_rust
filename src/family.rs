use std::iter::from_fn;

use crate::entity::Entity;

#[derive(Clone)]
pub struct Family<'a> {
    entity_root: usize,
    entity_index: usize,
    entity_instances: &'a [Entity],
    entity_indices: &'a [EntityIndices],
    segment_indices: &'a [SegmentIndices],
}

pub struct Families<'a> {
    entity_roots: &'a [(usize, usize)],
    entity_instances: &'a [Entity],
    entity_indices: &'a [EntityIndices],
    segment_indices: &'a [SegmentIndices],
}

pub struct FamiliesIterator<'a, 'b> {
    index: usize,
    families: &'b Families<'a>,
}

pub enum Direction {
    BottomUp,
    TopDown,
}

#[derive(Clone)]
pub struct EntityIndices {
    pub segment: usize,
    pub offset: usize,
    pub parent: Option<usize>,
    pub next: Option<usize>,
}

#[derive(Clone)]
pub struct SegmentIndices {
    pub segment: usize,
    pub count: usize,
    pub index: usize,
    pub store: usize,
}

impl<'a> Family<'a> {
    pub const EMPTY: Self = Self {
        entity_root: 0,
        entity_index: 0,
        entity_instances: &[],
        entity_indices: &[],
        segment_indices: &[],
    };

    #[inline]
    pub const fn new(
        entity_root: usize,
        entity_index: usize,
        entity_instances: &'a [Entity],
        entity_indices: &'a [EntityIndices],
        segment_indices: &'a [SegmentIndices],
    ) -> Self {
        Self {
            entity_root,
            entity_index,
            entity_instances,
            entity_indices,
            segment_indices,
        }
    }

    #[inline]
    pub fn entity(&self) -> Entity {
        let entity_indices = &self.entity_indices[self.entity_index];
        let segment_indices = &self.segment_indices[entity_indices.segment];
        let offset = segment_indices.count * self.entity_root + entity_indices.offset;
        self.entity_instances[segment_indices.index + offset]
    }

    pub fn parent(&self) -> Option<Self> {
        Some(self.with(self.entity_indices[self.entity_index].parent?))
    }

    pub fn root(&self) -> Self {
        // Do not assume that index '0' is the root since there might be multiple roots.
        self.parent()
            .map(|parent| parent.root())
            .unwrap_or(self.clone())
    }

    pub fn children(&self) -> impl Iterator<Item = Family<'a>> + '_ {
        let parent = Some(self.entity_index);
        let mut next = parent.map(|index| index + 1).filter(|&index| {
            index < self.entity_indices.len() && self.entity_indices[index].parent == parent
        });

        from_fn(move || {
            let current = next?;
            next = self.entity_indices[current].next;
            Some(self.with(current))
        })
    }

    pub fn descend(&self, direction: Direction, mut each: impl FnMut(Self)) {
        fn top_down<'a>(parent: &Family<'a>, each: &mut impl FnMut(Family<'a>)) {
            for child in parent.children() {
                each(child.clone());
                top_down(&child, each);
            }
        }

        fn bottom_up<'a>(parent: &Family<'a>, each: &mut impl FnMut(Family<'a>)) {
            for child in parent.children() {
                bottom_up(&child, each);
                each(child);
            }
        }

        match direction {
            Direction::TopDown => top_down(self, &mut each),
            Direction::BottomUp => bottom_up(self, &mut each),
        }
    }

    pub fn descendants(&self, direction: Direction) -> impl Iterator<Item = Family<'a>> {
        let mut descendants = Vec::new();
        self.descend(direction, |child| descendants.push(child));
        descendants.into_iter()
    }

    pub fn ancestors(&self) -> impl Iterator<Item = Family<'a>> + '_ {
        let mut next = self.entity_indices[self.entity_index].parent;
        from_fn(move || {
            let current = next?;
            next = self.entity_indices[current].parent;
            Some(self.with(current))
        })
    }

    pub fn siblings(&self) -> impl Iterator<Item = Family<'a>> + '_ {
        let parent = self.entity_indices[self.entity_index].parent;
        let mut next = parent.map(|index| index + 1).filter(|&index| {
            index < self.entity_indices.len() && self.entity_indices[index].parent == parent
        });

        from_fn(move || {
            while let Some(current) = next {
                next = self.entity_indices[current].next;
                if current != self.entity_index {
                    return Some(self.with(current));
                }
            }
            None
        })
    }

    fn with(&self, entity_index: usize) -> Self {
        Self::new(
            self.entity_root,
            entity_index,
            self.entity_instances,
            self.entity_indices,
            self.segment_indices,
        )
    }
}

impl std::fmt::Debug for Family<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entity = self.entity();
        let parent = self.parent().map(|parent| parent.entity());
        let children: Vec<_> = self.children().map(|child| child.entity()).collect();
        f.debug_struct("Family")
            .field("entity", &entity)
            .field("parent", &parent)
            .field("children", &children)
            .finish()
    }
}

impl<'a> Families<'a> {
    pub const EMPTY: Self = Self {
        entity_roots: &[],
        entity_instances: &[],
        entity_indices: &[],
        segment_indices: &[],
    };

    pub fn new(
        entity_roots: &'a [(usize, usize)],
        entity_instances: &'a [Entity],
        entity_indices: &'a [EntityIndices],
        segment_indices: &'a [SegmentIndices],
    ) -> Self {
        Self {
            entity_roots,
            entity_instances,
            entity_indices,
            segment_indices,
        }
    }
}

impl std::fmt::Debug for Families<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl<'a, 'b> IntoIterator for &'b Families<'a> {
    type Item = Family<'a>;
    type IntoIter = FamiliesIterator<'a, 'b>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            index: 0,
            families: self,
        }
    }
}

impl<'a, 'b> Iterator for FamiliesIterator<'a, 'b> {
    type Item = Family<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let pair = self.families.entity_roots.get(self.index)?;
        self.index += 1;
        Some(Family::new(
            pair.0,
            pair.1,
            self.families.entity_instances,
            self.families.entity_indices,
            self.families.segment_indices,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, 'b> ExactSizeIterator for FamiliesIterator<'a, 'b> {
    fn len(&self) -> usize {
        self.families.entity_roots.len() - self.index
    }
}
