use serde::Serialize;
use std::slice::{Iter, IterMut};

/// An ordered map.
/// Internally this is just two vectors -
/// one for keys and one for values.
#[derive(Debug, Clone, Serialize)]
pub struct Collection<TKey, TData> {
    keys: Vec<TKey>,
    values: Vec<TData>,
}

impl<TKey: PartialEq, TData> Collection<TKey, TData> {
    pub const fn new() -> Self {
        Self {
            keys: vec![],
            values: vec![],
        }
    }

    pub fn insert(&mut self, key: TKey, value: TData) {
        self.keys.push(key);
        self.values.push(value);

        assert_eq!(self.keys.len(), self.values.len());
    }

    pub fn get(&self, key: &TKey) -> Option<&TData> {
        let index = self.keys.iter().position(|k| k == key);
        match index {
            Some(index) => self.values.get(index),
            None => None,
        }
    }

    pub fn get_mut(&mut self, key: &TKey) -> Option<&mut TData> {
        let index = self.keys.iter().position(|k| k == key);
        match index {
            Some(index) => self.values.get_mut(index),
            None => None,
        }
    }

    pub fn remove(&mut self, key: &TKey) -> Option<TData> {
        assert_eq!(self.keys.len(), self.values.len());

        let index = self.keys.iter().position(|k| k == key);
        if let Some(index) = index {
            self.keys.remove(index);
            Some(self.values.remove(index))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn first(&self) -> Option<&TData> {
        self.values.first()
    }

    pub fn as_slice(&self) -> &[TData] {
        self.values.as_slice()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, TData> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, TData> {
        self.values.iter_mut()
    }
}

impl<TKey: PartialEq, TData> From<(TKey, TData)> for Collection<TKey, TData> {
    fn from((key, value): (TKey, TData)) -> Self {
        let mut collection = Self::new();
        collection.insert(key, value);
        collection
    }
}

impl<TKey: PartialEq, TData> FromIterator<(TKey, TData)> for Collection<TKey, TData> {
    fn from_iter<T: IntoIterator<Item = (TKey, TData)>>(iter: T) -> Self {
        let mut collection = Self::new();
        for (key, value) in iter {
            collection.insert(key, value);
        }

        collection
    }
}

impl<'a, TKey: PartialEq, TData> IntoIterator for &'a Collection<TKey, TData> {
    type Item = &'a TData;
    type IntoIter = CollectionIntoIterator<'a, TKey, TData>;

    fn into_iter(self) -> Self::IntoIter {
        CollectionIntoIterator {
            collection: self,
            index: 0,
        }
    }
}

pub struct CollectionIntoIterator<'a, TKey, TData> {
    collection: &'a Collection<TKey, TData>,
    index: usize,
}

impl<'a, TKey: PartialEq, TData> Iterator for CollectionIntoIterator<'a, TKey, TData> {
    type Item = &'a TData;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.collection.values.get(self.index);
        self.index += 1;
        res
    }
}

impl<TKey: PartialEq, TData> Default for Collection<TKey, TData> {
    fn default() -> Self {
        Self::new()
    }
}
