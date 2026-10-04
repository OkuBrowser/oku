use std::fmt::Debug;

// A commutative combine trait for updates
pub trait Combine: Debug {
    fn combine(self, other: Self) -> Self;
}

#[allow(dead_code)]
pub trait CombineInPlace: Combine {
    fn combine_with(&mut self, other: Self) -> Self;
    fn is_neutral(&self) -> bool;
}
