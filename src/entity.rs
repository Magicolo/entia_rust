use crate::*;

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
    pub const ZERO: Entity = Entity {
        index: 0,
        generation: 0,
    };
}

impl World {
    #[inline]
    pub(crate) fn get_data(&self, entity: Entity) -> Option<&Data> {
        self.data
            .get(entity.index as usize)
            .filter(|data| data.alive && data.generation == entity.generation)
    }

    #[inline]
    pub(crate) fn get_data_mut(&mut self, entity: Entity) -> Option<&mut Data> {
        self.data
            .get_mut(entity.index as usize)
            .filter(|data| data.alive && data.generation == entity.generation)
    }

    #[inline]
    pub fn has_entity(&self, entity: Entity) -> bool {
        self.get_data(entity).is_some()
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
        match self.get_data_mut(entity) {
            Some(data) => {
                data.alive = false;
                self.frozen_indices.push(entity.index);
                return true;
            }
            None => false,
        }
    }
}
