use inplace_vec_builder::InPlaceVecBuilder;

struct Collection {
    elements: Vec<u32>,
}

impl Collection {
    fn op(&mut self) {
        let mut res = self
            .elements
            .iter()
            .filter(|x| **x > 5)
            .map(|x| *x * 2)
            .chain(std::iter::once(123))
            .collect();
        std::mem::swap(&mut self.elements, &mut res);
    }

    fn inplace_op(&mut self) {
        let mut t = InPlaceVecBuilder::from(&mut self.elements);
        while let Some(elem) = t.pop_front() {
            if elem > 5 {
                t.push(elem * 2);
            }
        }
        t.push(123);
    }
}

fn main() {
    let mut x = Collection {
        elements: Vec::new(),
    };
    x.op();
    x.inplace_op();
}
