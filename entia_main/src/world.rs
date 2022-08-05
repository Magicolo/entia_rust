use crate::{identify, resources::Resources};

// Such a 'Link' would allow to compute which components have been added or removed.
/*
- Add 'Added/Removed<T>' query filters. The filters would hold a 'Bits' that represent the indices:
    fn dynamic_filter(state: &mut Self::State, index: usize) -> bool {
        state.bits.set(index, false) ????
    }
    - Will be equivalent to receiving a 'OnAdd<T>' message and 'query.get(onAdd.entity)'.
*/
// enum Link {
//     None,
//     Add { meta: usize, segment: usize },
//     Remove { meta: usize, segment: usize },
// }
pub struct World {
    identifier: usize,
    version: usize,
    resources: Resources,
}

impl World {
    pub fn new() -> Self {
        Self {
            identifier: identify(),
            version: 1,
            resources: Resources::default(),
        }
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub const fn version(&self) -> usize {
        self.version
    }

    #[inline]
    pub fn resources(&mut self) -> &mut Resources {
        &mut self.resources
    }

    #[inline]
    pub fn modify(&mut self) {
        self.version += 1;
    }
}
