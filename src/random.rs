use rand::{rngs::ThreadRng, Rng};

pub trait RemoveRandom {
    type Item;
    fn remove_random(&mut self, rng: &mut ThreadRng) -> Option<Self::Item>;
}

impl<T> RemoveRandom for Vec<T> {
    type Item = T;

    fn remove_random(&mut self, rng: &mut ThreadRng) -> Option<Self::Item> {
        match self.len() {
            0 => None,
            1 => Some(self.swap_remove(0)),
            r => Some(self.swap_remove(rng.gen_range(0..r))),
        }
    }
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

impl<'a, T> GetRandom for &'a Vec<T> {
    type Item = &'a T;

    fn get_random(self, rng: &mut ThreadRng) -> Option<Self::Item> {
        match self.len() {
            0 => None,
            1 => self.first(),
            len => self.get(rng.gen_range(0..len)),
        }
    }
}

pub type ItemAndWeight<T> = (T, f64);

#[derive(Clone)]
pub struct WeightedList<T> {
    items: Vec<ItemAndWeight<T>>,
    total_weight: f64,
}

///WeightedList which can only be accessed randomly.
///Interally the list is sorted by weight so that
///the number of average iterations during a search in minimized.
impl<T> WeightedList<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::default(),
            total_weight: 0.0,
        }
    }

    pub fn add(&mut self, item: impl Into<ItemAndWeight<T>>) {
        let item = item.into();
        let weight = item.1;

        assert!(
            weight >= 0.0,
            "item weight must be greater than or equal to zero, given: {weight}"
        );

        self.items.push(item);
        self.total_weight += weight;
    }

    pub fn get(&self, rng: &mut ThreadRng) -> Option<(&T, usize)> {
        match self.len() {
            0 => None,
            1 => self.items.first().map(|(val, _)| (val, 0)),
            _ => {
                let needle = rng.gen_range(0.0..self.total_weight);
                let mut running_total = 0.0;

                for (i, (item, weight)) in self.items.iter().enumerate() {
                    running_total += *weight;
                    if needle < running_total {
                        return Some((item, i));
                    }
                }

                panic!("Reached end without finding match");
            }
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            total_weight: 0.0,
        }
    }

    pub fn change_weight(&mut self, index: usize, weight: f64) {
        assert!(
            weight >= 0.0,
            "item weight must be greater than or equal to zero, given: {weight}"
        );

        let item = &mut self.items[index];
        let old_weight = item.1;
        self.total_weight = (self.total_weight - old_weight) + weight;
        item.1 = weight;
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter<'a>(&'a self, rng: &'a mut ThreadRng) -> WeightedListIterator<'a, T> {
        WeightedListIterator::new(self, rng)
    }
}

impl<T> Default for WeightedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FromIterator<ItemAndWeight<T>> for WeightedList<T> {
    fn from_iter<I: IntoIterator<Item = (T, f64)>>(iter: I) -> Self {
        let iter = iter.into_iter();

        let mut list = {
            let (lower_bound, _) = iter.size_hint();
            Self::with_capacity(lower_bound)
        };

        for item_weight in iter {
            list.add(item_weight);
        }

        list
    }
}

impl<'a, T> GetRandom for &'a WeightedList<T> {
    type Item = (&'a T, usize);

    fn get_random(self, rng: &mut ThreadRng) -> Option<Self::Item> {
        self.get(rng)
    }
}

pub struct WeightedListIterator<'a, T> {
    list: &'a WeightedList<T>,
    seen: Vec<usize>,
    remaining_weight: f64,
    rng: &'a mut ThreadRng,
}

impl<'a, T> WeightedListIterator<'a, T> {
    fn new(list: &'a WeightedList<T>, rng: &'a mut ThreadRng) -> Self {
        Self {
            list,
            seen: Vec::with_capacity(10),
            remaining_weight: list.total_weight,
            rng,
        }
    }
}

impl<'a, T> Iterator for WeightedListIterator<'a, T>
where
    T: PartialEq,
{
    type Item = (&'a T, usize);

    fn next(&mut self) -> Option<Self::Item> {
        match self.list.len() - self.seen.len() {
            0 => None,
            1 => {
                let (item, i) = self
                    .list
                    .items
                    .iter()
                    .enumerate()
                    .find(|(i, _)| !self.seen.contains(i))
                    .map(|(i, (ref item, _))| (item, i))
                    .unwrap();

                self.seen.push(i);
                Some((item, i))
            }
            _ => {
                let needle = self.rng.gen_range(0.0..self.remaining_weight);
                let mut running_total = 0.0;

                for (i, (item, weight)) in self.list.items.iter().enumerate() {
                    if self.seen.contains(&i) {
                        continue;
                    }

                    running_total += weight;
                    if needle < running_total {
                        self.remaining_weight -= weight;
                        self.seen.push(i);
                        return Some((item, i));
                    }
                }

                panic!("Reached end without finding match");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use rand::{rngs::ThreadRng, thread_rng, Rng};

    use crate::random::{IntoIterShuffled, WeightedList};

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

    #[test]
    fn test_weighted_list() {
        let rng = &mut rand::thread_rng();

        let mut list = WeightedList::default();
        assert!(list.clone().iter(rng).next().is_none());

        list.add((1, 1.0));
        for _ in 0..10 {
            let mut iter = list.iter(rng);
            assert!(matches!(iter.next(), Some((&1, 0))));
            assert!(iter.next().is_none());
        }

        list.add((2, 2.0));
        let mut seen = (0, 0);
        const TOTAL: usize = 1000;
        for _ in 0..TOTAL {
            list.iter(rng)
                .enumerate()
                .for_each(|(i, (v, _))| if *v == 1 { seen.0 += i } else { seen.1 += i })
        }

        let (min, max) = seen.min_max();
        assert!(
            *min as f64 / TOTAL as f64 > 0.3,
            "{min} is not around 33% of {TOTAL}"
        );
        assert!(
            *max as f64 / TOTAL as f64 > 0.6,
            "{max} is not around 66% of {TOTAL}"
        );

        list.add((3, 3.0));
        let mut seen = (0, 0);
        for _ in 0..TOTAL {
            list.iter(rng)
                .enumerate()
                .for_each(|(i, (v, _))| if *v == 3 { seen.0 += i } else { seen.1 += i })
        }

        let (min, _) = seen.min_max();
        assert!(
            *min as f64 / TOTAL as f64 > 0.45,
            "{min} is not around 50% of {TOTAL}"
        );
    }

    #[test]
    fn bench_weighted_list() {
        impl<T> WeightedList<T> {
            fn get_mut(&mut self, rng: &mut ThreadRng) -> Option<(&mut T, usize)> {
                match self.len() {
                    0 => None,
                    1 => self.items.first_mut().map(|(val, _)| (val, 0)),
                    _ => {
                        let needle = rng.gen_range(0.0..self.total_weight);
                        let mut running_total = 0.0;

                        for (i, (item, weight)) in self.items.iter_mut().enumerate() {
                            running_total += *weight;
                            if needle < running_total {
                                return Some((item, i));
                            }
                        }

                        panic!("Reached end without finding match");
                    }
                }
            }

            fn change_weight_bench(&mut self, mut index: usize, weight: f64) {
                assert!(
                    weight >= 0.0,
                    "item weight must be greater than or equal to zero, given: {weight}"
                );

                let item = &mut self.items[index];
                let old_weight = item.1;
                self.total_weight = (self.total_weight - old_weight) + weight;
                item.1 = weight;

                //NB: Benchmarking that this is slower than not sorting
                if old_weight < weight {
                    //Bubble up
                    while index > 0 && self.items[index - 1].1 < weight {
                        self.items.swap(index, index - 1);
                        index -= 1;
                    }
                } else {
                    //Bubble down
                    let max = self.len() - 1;
                    while index < max && self.items[index + 1].1 > weight {
                        self.items.swap(index, index + 1);
                        index += 1;
                    }
                }
            }
        }

        #[derive(Clone)]
        struct W(usize);

        let list = (0..100)
            .map(|_| (W(20), 1.0 / (20 + 1) as f64))
            .collect::<WeightedList<_>>();

        let rng = &mut thread_rng();

        let time_current = {
            let start = Instant::now();

            for _ in 0..10_000 {
                let mut list = list.clone();
                for _ in 0..200 {
                    let (item, index) = list.get_mut(rng).unwrap();
                    if rng.gen_range(0..100) <= 80 {
                        item.0 += 1;
                        let denom = (item.0 + 1) as f64;
                        list.change_weight(index, 1.0 / denom);
                    }
                }
            }

            start.elapsed()
        };

        let time_bench = {
            let start = Instant::now();

            for _ in 0..10_000 {
                let mut list = list.clone();
                for _ in 0..200 {
                    let (item, index) = list.get_mut(rng).unwrap();
                    if rng.gen_range(0..100) <= 80 {
                        item.0 += 1;
                        let denom = (item.0 + 1) as f64;
                        list.change_weight_bench(index, 1.0 / denom);
                    }
                }
            }

            start.elapsed()
        };

        assert!(
            time_current < time_bench,
            "Current is not faster! Current: {}, Bench: {}",
            time_current.as_millis(),
            time_bench.as_millis()
        );
    }
}
