use rand::{rngs::ThreadRng, Rng};

use crate::random::{GetRandom, RemoveRandom};

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

    fn get_index(&self, rng: &mut ThreadRng) -> Option<usize> {
        match self.len() {
            0 => None,
            1 => Some(0),
            _ => {
                let needle = rng.gen_range(0.0..self.total_weight);
                let mut running_total = 0.0;

                for (i, (_, weight)) in self.items.iter().enumerate() {
                    running_total += *weight;
                    if needle < running_total {
                        return Some(i);
                    }
                }

                panic!("Reached end without finding match");
            }
        }
    }

    pub fn get(&self, rng: &mut ThreadRng) -> Option<(&T, usize)> {
        self.get_index(rng).map(|index| {
            let (item, _) = self.items.get(index).expect("Unable to find item at index");
            (item, index)
        })
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

impl<T> RemoveRandom for WeightedList<T> {
    type Item = (ItemAndWeight<T>, usize);

    fn remove_random(&mut self, rng: &mut ThreadRng) -> Option<Self::Item> {
        self.get_index(rng).map(|index| {
            let item = self.items.swap_remove(index);
            self.total_weight -= item.1;
            (item, index)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use rand::{rngs::ThreadRng, Rng};

    use crate::random::IntoIterShuffled;

    use super::WeightedList;

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

    #[derive(Clone, PartialEq, Eq)]
    struct W(usize);

    #[test]
    fn bench_weighted_list_change_weight() {
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

                //NOTE: Benchmarking that this is slower than not sorting
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

        let list = (0..200)
            .map(|_| (W(20), 1.0 / (20 + 1) as f64))
            .collect::<WeightedList<_>>();
        let rng = &mut rand::thread_rng();

        let time_current = {
            let start = Instant::now();

            for _ in 0..1000 {
                let mut list = list.clone();
                for _ in 0..200 {
                    let (item, index) = list.get_mut(rng).expect("Unable to get item mutably");
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

            for _ in 0..1000 {
                let mut list = list.clone();
                for _ in 0..200 {
                    let (item, index) = list.get_mut(rng).expect("Unable to get item mutably");
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

    #[test]
    fn bench_weighted_list_iterator() {
        struct WeightedListIterator<'a, T> {
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
                            .expect("Unable to find not-yet-seen index");

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

        impl<T> WeightedList<T> {
            fn iter<'a>(&'a self, rng: &'a mut ThreadRng) -> WeightedListIterator<'a, T> {
                WeightedListIterator::new(self, rng)
            }
        }

        let list = (0..2000)
            .map(|i: usize| ((i, i, i, i), 1.0 / 20.0))
            .collect::<WeightedList<_>>();
        let rng = &mut rand::thread_rng();

        let time_current = {
            let start = Instant::now();

            for _ in 0..5000 {
                let _ = list.clone().into_iter_shuffled(rng).take(10).count();
            }

            start.elapsed()
        };

        let time_bench = {
            let start = Instant::now();

            //NOTE: Apparently the WeightedListIterator is at least 5x slower! 🤔
            for _ in 0..1000 {
                let _ = list.iter(rng).take(10).count();
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