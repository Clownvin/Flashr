use rand::{rngs::ThreadRng, Rng};

pub trait IterShuffled<'rng>
where
    Self: IntoIterator,
{
    fn iter_shuffled(self, rng: &'rng mut ThreadRng) -> ShuffleIter<'rng, Self::Item>;
}

pub struct ShuffleIter<'rng, T> {
    values: Vec<T>,
    rng: &'rng mut ThreadRng,
}

impl<T> Iterator for ShuffleIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.values.len() {
            0 => None,
            1 => Some(self.values.swap_remove(0)),
            r => Some(self.values.swap_remove(self.rng.gen_range(0..r))),
        }
    }
}

impl<'rng, T> IterShuffled<'rng> for Vec<T> {
    fn iter_shuffled(self, rng: &'rng mut ThreadRng) -> ShuffleIter<'rng, Self::Item> {
        ShuffleIter { values: self, rng }
    }
}

pub trait GetRandom {
    type Item;

    fn get_random(&self, rng: &mut ThreadRng) -> Option<&'_ Self::Item>;
}

impl<T> GetRandom for [T] {
    type Item = T;

    fn get_random(&self, rng: &mut ThreadRng) -> Option<&'_ Self::Item> {
        self.get(rng.gen_range(0..self.len()))
    }
}
