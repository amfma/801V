//! A small, fast, non-cryptographic hasher (the "Fx" multiply-rotate scheme used
//! by rustc), implemented here so the crate keeps zero dependencies.
//!
//! The interpreter's hash maps are keyed by short identifier strings and token
//! indices that come from the program being run, never from an attacker-facing
//! network service, so the default SipHash's DoS resistance is not needed and
//! its cost showed up as roughly a quarter of the runtime in profiles.
use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
};

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while rest.len() >= 8 {
            self.add(u64::from_le_bytes(rest[..8].try_into().unwrap()));
            rest = &rest[8..];
        }
        if rest.len() >= 4 {
            self.add(u32::from_le_bytes(rest[..4].try_into().unwrap()) as u64);
            rest = &rest[4..];
        }
        for &byte in rest {
            self.add(byte as u64);
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(i as u64);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add(i as u64);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub type FxBuildHasher = BuildHasherDefault<FxHasher>;

/// A `HashMap` using [`FxHasher`]. Create with `FxHashMap::default()`.
pub type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaves_like_a_normal_map() {
        let mut map: FxHashMap<String, i32> = FxHashMap::default();
        for (i, key) in ["a", "b", "counter", "a_much_longer_identifier_name", ""].iter().enumerate() {
            map.insert(key.to_string(), i as i32);
        }
        assert_eq!(map.get("a"), Some(&0));
        assert_eq!(map.get("counter"), Some(&2));
        assert_eq!(map.get("a_much_longer_identifier_name"), Some(&3));
        assert_eq!(map.get(""), Some(&4));
        assert_eq!(map.get("missing"), None);
        assert_eq!(map.len(), 5);
    }

    #[test]
    fn works_for_integer_keys() {
        let mut map: FxHashMap<usize, usize> = FxHashMap::default();
        for i in 0..1000 {
            map.insert(i, i * 2);
        }
        assert!((0..1000).all(|i| map.get(&i) == Some(&(i * 2))));
    }

    #[test]
    fn similar_strings_hash_differently() {
        use std::hash::BuildHasher;
        let build = FxBuildHasher::default();
        let hashes: Vec<u64> = ["x1", "x2", "x3", "y1", "xx", "x"].iter().map(|s| build.hash_one(s)).collect();
        let unique: std::collections::HashSet<_> = hashes.iter().collect();
        assert_eq!(unique.len(), hashes.len());
    }
}
