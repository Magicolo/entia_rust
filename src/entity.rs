use crate::world::Inner;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

pub(crate) struct Data {
    pub generation: u32,
    pub alive: bool,
    pub segment: u32,
    pub index: u32,
}

impl Entity {
    pub const ZERO: Self = Self::new(0, 0);

    #[inline]
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

impl Inner {
    #[inline]
    pub(crate) fn get_entity_data(data: &Vec<Data>, entity: Entity) -> Option<&Data> {
        data.get(entity.index as usize)
            .filter(|data| data.alive && data.generation == entity.generation)
    }

    #[inline]
    pub(crate) fn get_data_mut(data: &mut Vec<Data>, entity: Entity) -> Option<&mut Data> {
        data.get_mut(entity.index as usize)
            .filter(|data| data.alive && data.generation == entity.generation)
    }

    pub fn resolve_entities(&mut self) {
        self.free_indices.append(&mut self.frozen_indices);
    }

    #[inline]
    pub fn has_entity(&self, entity: Entity) -> bool {
        Self::get_entity_data(&self.data, entity).is_some()
    }

    pub fn create_entity(&mut self) -> Entity {
        match self.free_indices.pop() {
            Some(index) => {
                let data = &mut self.data[index as usize];
                let generation = data.generation + 1;
                *data = Data {
                    generation,
                    alive: true,
                    segment: 0,
                    index: 0,
                };
                Entity { index, generation }
            }
            None => {
                let index = self.data.len() as u32;
                let generation = 1;
                self.data.push(Data {
                    generation,
                    alive: true,
                    segment: 0,
                    index: 0,
                });
                Entity { index, generation }
            }
        }
    }

    pub fn destroy_entity(&mut self, entity: Entity) -> bool {
        match Self::get_data_mut(&mut self.data, entity) {
            Some(data) => {
                data.alive = false;
                self.frozen_indices.push(entity.index);
                return true;
            }
            None => false,
        }
    }
}
