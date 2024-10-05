use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    pub id: u32,
    pub generation: u32,
}

impl Entity {
    pub const ZERO: Entity = Entity {
        id: 0,
        generation: 0,
    };

    #[inline]
    pub const fn new(id: u32) -> Self {
        Self { id, generation: 0 }
    }

    #[inline]
    pub fn with_generation(mut self, generation: u32) -> Self {
        self.generation = generation;
        self
    }
}

#[derive(Debug)]
pub struct Entities {
    current: u32,
    generations: HashMap<u32, u32>,
    free: Vec<u32>,
}

impl Entities {
    pub fn new() -> Entities {
        Entities {
            current: 0,
            generations: HashMap::new(),
            free: Vec::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let id = if let Some(id) = self.free.pop() {
            id
        } else {
            let id = self.current;
            self.current += 1;
            self.generations.insert(id, 0);
            id
        };

        let generation = self.generations.get(&id).copied().unwrap_or(0);
        Entity { id, generation }
    }

    pub fn despawn(&mut self, entity: &Entity) -> bool {
        if let Some(gen) = self.generations.get(&entity.id) {
            if *gen == entity.generation {
                self.free.push(entity.id);
                self.generations.insert(entity.id, gen + 1);
                return true;
            }
        }

        false
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &u32)> + '_ {
        self.generations.iter().map(|(id, gen)| (id, gen))
    }
}
