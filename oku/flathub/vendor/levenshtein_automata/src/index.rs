use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

pub(crate) struct Index<I: Eq + Hash + Clone> {
    index: HashMap<I, u32>,
    items: Vec<I>,
}

impl<I: Eq + Hash + Clone + Debug> Index<I> {
    pub fn new() -> Index<I> {
        Index {
            index: HashMap::new(),
            items: Vec::new(),
        }
    }

    pub fn get_or_allocate(&mut self, item: &I) -> u32 {
        let index_len: u32 = self.len();
        let item_index = *self.index.entry(item.clone()).or_insert(index_len);
        if item_index == index_len {
            self.items.push(item.clone());
        }
        item_index as u32
    }

    pub fn len(&self) -> u32 {
        self.items.len() as u32
    }

    pub fn get_from_id(&self, id: u32) -> &I {
        &self.items[id as usize]
    }
}
