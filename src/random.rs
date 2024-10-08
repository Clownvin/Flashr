/*
 * Copyright (C) 2024 Clownvin <123clownvin@gmail.com>
 *
 * This file is part of Flashr.
 *
 * Flashr is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Flashr is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Flashr.  If not, see <http://www.gnu.org/licenses/>.
 */

use rand::{rngs::ThreadRng, Rng};

pub trait RandomIndex {
    fn random_index(&self, rng: &mut ThreadRng) -> Option<usize>;
}

pub trait RemoveRandom {
    type Item;
    fn remove_random(&mut self, rng: &mut ThreadRng) -> Option<Self::Item>;
}

pub trait IntoIterShuffled<'rng, C>
where
    C: RemoveRandom,
{
    fn into_iter_shuffled(self, rng: &'rng mut ThreadRng) -> IntoShuffleIter<'rng, C>;
}

pub struct IntoShuffleIter<'rng, C>
where
    C: RemoveRandom,
{
    values: C,
    rng: &'rng mut ThreadRng,
}

impl<C> Iterator for IntoShuffleIter<'_, C>
where
    C: RemoveRandom,
{
    type Item = C::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.values.remove_random(self.rng)
    }
}

impl<'rng, C> IntoIterShuffled<'rng, C> for C
where
    C: RemoveRandom,
{
    fn into_iter_shuffled(self, rng: &'rng mut ThreadRng) -> IntoShuffleIter<'rng, C> {
        IntoShuffleIter { values: self, rng }
    }
}

pub trait GetRandom {
    type Item;

    fn get_random(self, rng: &mut ThreadRng) -> Option<Self::Item>;
}

impl<T> RandomIndex for Vec<T> {
    fn random_index(&self, rng: &mut ThreadRng) -> Option<usize> {
        match self.len() {
            0 => None,
            1 => Some(0),
            r => Some(rng.gen_range(0..r)),
        }
    }
}

impl<T> RemoveRandom for Vec<T> {
    type Item = T;

    fn remove_random(&mut self, rng: &mut ThreadRng) -> Option<Self::Item> {
        self.random_index(rng).map(|index| self.swap_remove(index))
    }
}

impl<'a, T> GetRandom for &'a Vec<T> {
    type Item = &'a T;
    fn get_random(self, rng: &mut ThreadRng) -> Option<Self::Item> {
        self.random_index(rng).and_then(|index| self.get(index))
    }
}

#[cfg(test)]
mod tests {
    use super::{GetRandom, IntoIterShuffled};

    trait MinMax<T> {
        fn min_max(&self) -> (&T, &T);
    }

    impl<T: PartialOrd> MinMax<T> for (T, T) {
        fn min_max(&self) -> (&T, &T) {
            let (l, r) = self;
            if l < r {
                (l, r)
            } else {
                (r, l)
            }
        }
    }

    #[test]
    fn test_get_random() {
        let rng = &mut rand::thread_rng();

        let mut vals = vec![];
        assert!(vals.get_random(rng).is_none());

        vals.push(1);
        for _ in 0..10 {
            assert!(matches!(vals.get_random(rng), Some(1)));
        }

        vals.push(2);
        let mut seen = (0, 0);
        const TOTAL: usize = 1000;
        for _ in 0..TOTAL {
            let val = vals.get_random(rng).expect("Unable to get random");
            if *val == 1 {
                seen.0 += 1;
            } else {
                seen.1 += 1;
            }
        }

        let (min, _) = seen.min_max();
        assert!(
            *min as f64 / TOTAL as f64 > 0.45,
            "{min} is not around half of {TOTAL}"
        );
    }

    #[test]
    fn test_iter_shuffled() {
        let rng = &mut rand::thread_rng();

        let mut vals = vec![];
        assert!(vals.clone().into_iter_shuffled(rng).next().is_none());

        vals.push(1);
        for _ in 0..10 {
            let mut iter = vals.clone().into_iter_shuffled(rng);
            assert!(matches!(iter.next(), Some(1)));
            assert!(iter.next().is_none());
        }

        vals.push(2);
        let mut seen = (0, 0);
        const TOTAL: usize = 1000;
        for _ in 0..TOTAL {
            vals.clone()
                .into_iter_shuffled(rng)
                .enumerate()
                .for_each(|(i, v)| if v == 1 { seen.0 += i } else { seen.1 += i })
        }

        let (min, _) = seen.min_max();
        assert!(
            *min as f64 / TOTAL as f64 > 0.45,
            "{min} is not around half of {TOTAL}"
        );
    }
}
