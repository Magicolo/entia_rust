use crate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

pub trait Entities {
    fn exists(&self, entity: Entity) -> bool;
    fn create(&mut self) -> Entity;
    fn destroy(&mut self, entity: Entity) -> bool;
}

pub(crate) struct Data {
    pub generation: u32,
    pub alive: bool,
    pub segment: u32,
    pub index: u32,
}

impl World {
    #[inline]
    pub(crate) fn get_data(&self, entity: Entity) -> Option<&Data> {
        self.entities
            .get(entity.index as usize)
            .filter(|data| data.alive && data.generation == entity.generation)
    }

    #[inline]
    pub(crate) fn get_data_mut(&mut self, entity: Entity) -> Option<&mut Data> {
        self.entities
            .get_mut(entity.index as usize)
            .filter(|data| data.alive && data.generation == entity.generation)
    }
}

impl Entities for World {
    #[inline]
    fn exists(&self, entity: Entity) -> bool {
        self.get_data(entity).is_some()
    }

    fn create(&mut self) -> Entity {
        match self.free_indices.pop() {
            Some(index) => {
                let data = &mut self.entities[index as usize];
                let generation = data.generation + 1;
                *data = Data {
                    generation,
                    alive: true,
                    segment: todo!(),
                    index: todo!(),
                };
                Entity { index, generation }
            }
            None => {
                let index = self.entities.len() as u32;
                let generation = 1;
                self.entities.push(Data {
                    generation,
                    alive: true,
                    segment: todo!(),
                    index: todo!(),
                });
                Entity { index, generation }
            }
        }
    }

    fn destroy(&mut self, entity: Entity) -> bool {
        match self.get_data_mut(entity) {
            Some(data) => {
                data.alive = false;
                self.free_indices.push(entity.index);
                return true;
            }
            None => false,
        }
    }
}
