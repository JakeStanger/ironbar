use serde::Serialize;
use std::slice::{Iter, IterMut};
use std::vec;

/// An ordered map.
/// Internally this is just two vectors -
/// one for keys and one for values.
#[derive(Debug, Clone, Serialize)]
pub struct Collection<TKey, TData> {
    keys: Vec<TKey>,
    values: Vec<TData>,
}

impl<TKey: PartialEq, TData> Collection<TKey, TData> {
    /// Creates a new empty collection.
    pub const fn new() -> Self {
        Self {
            keys: vec![],
            values: vec![],
        }
    }

    /// Inserts a new key/value pair at the end of the collection.
    pub fn insert(&mut self, key: TKey, value: TData) {
        self.keys.push(key);
        self.values.push(value);

        assert_eq!(self.keys.len(), self.values.len());
    }

    /// Gets a reference of the value for the specified key
    /// if it exists in the collection.
    pub fn get(&self, key: &TKey) -> Option<&TData> {
        let index = self.keys.iter().position(|k| k == key);
        match index {
            Some(index) => self.values.get(index),
            None => None,
        }
    }

    /// Gets a mutable reference for the value with the specified key
    /// if it exists in the collection.
    pub fn get_mut(&mut self, key: &TKey) -> Option<&mut TData> {
        let index = self.keys.iter().position(|k| k == key);
        match index {
            Some(index) => self.values.get_mut(index),
            None => None,
        }
    }

    /// Checks if a value for the given key exists inside the collection
    pub fn contains(&self, key: &TKey) -> bool {
        self.keys.contains(key)
    }

    /// Removes the key/value from the collection
    /// if it exists
    /// and returns the removed value.
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

    /// Gets the length of the collection.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Gets a reference to the first value in the collection.
    pub fn first(&self) -> Option<&TData> {
        self.values.first()
    }

    /// Gets the values as a slice.
    pub fn as_slice(&self) -> &[TData] {
        self.values.as_slice()
    }

    /// Checks whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Gets an iterator for the collection.
    pub fn iter(&self) -> Iter<'_, TData> {
        self.values.iter()
    }

    /// Gets a mutable iterator for the collection
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

impl<TKey: PartialEq, TData> IntoIterator for Collection<TKey, TData> {
    type Item = TData;
    type IntoIter = vec::IntoIter<TData>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}
