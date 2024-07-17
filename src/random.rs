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

type ItemAndWeight<T> = (T, f64);

impl<T> From<WeightedItem<T>> for ItemAndWeight<T> {
    fn from(value: WeightedItem<T>) -> Self {
        (value.item, value.weight)
    }
}

#[derive(Clone)]
struct WeightedList<T> {
    items: Vec<WeightedItem<T>>,
    total_weight: f64,
}

impl<T> WeightedList<T> {
    fn new() -> Self {
        Self {
            items: Vec::default(),
            total_weight: 0.0,
        }
    }

    fn add(&mut self, item: impl Into<WeightedItem<T>>) {
        let item = item.into();
        let weight = item.weight;
        assert!(
            weight >= 0.0,
            "item weight must be greater than or equal to zero, given: {weight}"
        );
        self.total_weight += weight;
        self.items.push(item);
    }

    fn _remove(&mut self, item: &T) -> Option<ItemAndWeight<T>>
    where
        T: PartialEq,
    {
        let (item_index, _) = self
            .items
            .iter()
            .enumerate()
            .find(|(_, weighted)| &weighted.item == item)?;
        Some(self.items.swap_remove(item_index).into())
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            total_weight: 0.0,
        }
    }

    fn _get(&self, rng: &mut ThreadRng) -> &T {
        let val: f64 = rng.gen();
        let mut running_total = 0.0;

        for weighted in self.items.iter() {
            running_total += weighted.weight / self.total_weight;
            if val < running_total {
                return &weighted.item;
            }
        }

        panic!("Reached end without finding a match!");
    }

    fn _get_mut<'a>(&'a mut self, rng: &mut ThreadRng) -> (&T, Box<dyn FnOnce(f64) + 'a>) {
        let val: f64 = rng.gen();
        let mut running_total = 0.0;

        for weighted in self.items.iter_mut() {
            running_total += weighted.weight / self.total_weight;
            if val < running_total {
                return (
                    &weighted.item,
                    Box::new(|new_weight| {
                        assert!(
                            new_weight >= 0.0,
                            "Item weight must be greater than or equal to zero, given: {new_weight}"
                        );

                        self.total_weight = (self.total_weight - weighted.weight) + new_weight;
                        weighted.weight = new_weight;
                    }),
                );
            }
        }

        panic!("Reached end without finding a match!");
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

impl<T> RemoveRandom for &mut WeightedList<T> {
    type Item = ItemAndWeight<T>;

    fn remove_random(&mut self, rng: &mut ThreadRng) -> Option<Self::Item> {
        match self.len() {
            0 => None,
            1 => {
                let item = self.items.swap_remove(0);
                self.total_weight -= item.weight;
                Some(item.into())
            }
            _ => {
                let val: f64 = rng.gen();
                let mut running_total = 0.0;

                for (i, item) in self.items.iter_mut().enumerate() {
                    running_total += item.weight / self.total_weight;
                    if val < running_total {
                        let item = self.items.swap_remove(i);
                        self.total_weight -= item.weight;
                        return Some(item.into());
                    }
                }

                panic!("Reached end without finding a match!");
            }
        }
    }
}

#[derive(Clone)]
struct WeightedItem<T> {
    item: T,
    weight: f64,
}

impl<T> From<ItemAndWeight<T>> for WeightedItem<T> {
    fn from((item, weight): ItemAndWeight<T>) -> Self {
        Self { item, weight }
    }
}

#[cfg(test)]
mod tests {
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
        assert!(list.into_iter_shuffled(rng).next().is_none());

        list.add((1, 1.0));
        for _ in 0..10 {
            let mut list = list.clone();
            let mut iter = list.into_iter_shuffled(rng);
            assert!(matches!(iter.next(), Some((1, 1.0))));
            assert!(iter.next().is_none());
        }

        list.add((2, 2.0));
        let mut seen = (0, 0);
        const TOTAL: usize = 1000;
        for _ in 0..TOTAL {
            list.clone()
                .into_iter_shuffled(rng)
                .enumerate()
                .for_each(|(i, (v, _))| if v == 1 { seen.0 += i } else { seen.1 += i })
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
            list.clone()
                .into_iter_shuffled(rng)
                .enumerate()
                .for_each(|(i, (v, _))| if v == 3 { seen.0 += i } else { seen.1 += i })
        }

        let (min, _) = seen.min_max();
        assert!(
            *min as f64 / TOTAL as f64 > 0.45,
            "{min} is not around 50% of {TOTAL}"
        );
    }
}
