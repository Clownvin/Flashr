use rand::{rngs::ThreadRng, Rng};

pub trait IntoIterShuffled<'rng>
where
    Self: IntoIterator,
{
    fn into_iter_shuffled(self, rng: &'rng mut ThreadRng) -> IntoShuffleIter<'rng, Self::Item>;
}

pub struct IntoShuffleIter<'rng, T> {
    values: Vec<T>,
    rng: &'rng mut ThreadRng,
}

impl<T> Iterator for IntoShuffleIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.values.len() {
            0 => None,
            1 => Some(self.values.swap_remove(0)),
            r => Some(self.values.swap_remove(self.rng.gen_range(0..r))),
        }
    }
}

impl<'rng, T> IntoIterShuffled<'rng> for Vec<T> {
    fn into_iter_shuffled(self, rng: &'rng mut ThreadRng) -> IntoShuffleIter<'rng, Self::Item> {
        IntoShuffleIter { values: self, rng }
    }
}

pub trait GetRandom {
    type Item;

    fn get_random(&self, rng: &mut ThreadRng) -> Option<&'_ Self::Item>;
}

impl<T> GetRandom for [T] {
    type Item = T;

    fn get_random(&self, rng: &mut ThreadRng) -> Option<&'_ Self::Item> {
        match self.len() {
            0 => None,
            1 => self.first(),
            len => self.get(rng.gen_range(0..len)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::random::IntoIterShuffled;

    use super::GetRandom;

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
            let val = vals.get_random(rng).unwrap();
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
