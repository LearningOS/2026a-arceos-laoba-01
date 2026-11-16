//! Collection types.
//!
//! This module re-exports the collections provided by `alloc` and adds a
//! [`HashMap`], which `alloc` does not provide.

#[doc(no_inline)]
pub use alloc::collections::*;

use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

/// The default hasher used by [`HashMap`].
///
/// It is a 64-bit FNV-1a hasher seeded with random data from `axhal`, so the
/// bucket layout differs between boots.
struct DefaultHasher {
    state: u64,
}

impl DefaultHasher {
    fn new(seed: u64) -> Self {
        Self {
            state: 0xcbf2_9ce4_8422_2325 ^ seed,
        }
    }
}

impl Hasher for DefaultHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

fn random_seed() -> u64 {
    let seed = arceos_api::modules::axhal::misc::random() as u64;
    if seed == 0 { 1 } else { seed }
}

/// A hash map implemented with separate chaining.
///
/// The interface is similar to [`std::collections::HashMap`], but it uses a
/// fixed hasher seeded with random data instead of a randomly keyed `SipHash`.
pub struct HashMap<K, V> {
    buckets: Vec<Vec<(K, V)>>,
    len: usize,
    seed: u64,
}

impl<K, V> HashMap<K, V> {
    /// Creates an empty `HashMap`.
    pub fn new() -> Self {
        Self {
            buckets: Vec::new(),
            len: 0,
            seed: random_seed(),
        }
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Clears the map, removing all key-value pairs.
    pub fn clear(&mut self) {
        self.buckets.clear();
        self.len = 0;
    }

    fn hash(&self, key: &K) -> usize
    where
        K: Hash,
    {
        let mut hasher = DefaultHasher::new(self.seed);
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    fn index(&self, key: &K) -> usize
    where
        K: Hash,
    {
        self.hash(key) & (self.buckets.len() - 1)
    }

    fn resize(&mut self, capacity: usize)
    where
        K: Hash,
    {
        let mut buckets = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buckets.push(Vec::new());
        }

        for bucket in core::mem::replace(&mut self.buckets, buckets) {
            for (key, value) in bucket {
                let index = self.index(&key);
                self.buckets[index].push((key, value));
            }
        }
    }
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    /// Inserts a key-value pair into the map.
    ///
    /// If the map already had this key present, the old value is returned.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.buckets.is_empty() {
            self.resize(16);
        } else if self.len + 1 > self.buckets.len() * 3 / 4 {
            self.resize(self.buckets.len() * 2);
        }

        let index = self.index(&key);
        for (k, v) in self.buckets[index].iter_mut() {
            if *k == key {
                return Some(core::mem::replace(v, value));
            }
        }

        self.buckets[index].push((key, value));
        self.len += 1;
        None
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get(&self, key: &K) -> Option<&V> {
        if self.buckets.is_empty() {
            return None;
        }

        self.buckets[self.index(key)]
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if self.buckets.is_empty() {
            return None;
        }

        let index = self.index(key);
        self.buckets[index]
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Returns `true` if the map contains a value for the specified key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Removes a key from the map, returning the value if the key was present.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if self.buckets.is_empty() {
            return None;
        }

        let index = self.index(key);
        let bucket = &mut self.buckets[index];
        let position = bucket.iter().position(|(k, _)| k == key)?;
        self.len -= 1;
        Some(bucket.swap_remove(position).1)
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.buckets
            .iter()
            .flat_map(|bucket| bucket.iter().map(|(k, v)| (k, v)))
    }

    /// An iterator visiting all keys in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.iter().map(|(k, _)| k)
    }

    /// An iterator visiting all values in arbitrary order.
    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.iter().map(|(_, v)| v)
    }
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq, V> core::iter::FromIterator<(K, V)> for HashMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}
