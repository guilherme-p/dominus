// #![feature(test)]
#![feature(let_chains)]

use parking_lot::RwLock;
use parking_lot::RwLockUpgradableReadGuard;
use std::error::Error;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicUsize, Ordering};
use ahash::AHasher;
use std::hash::{Hash, Hasher};
use anyhow::{Result, anyhow};

pub struct Dominus<K, V> {
    entries: Vec<RwLock<Option<Entry<K, V>>>>,
    size: AtomicUsize,
    capacity: usize,
    max_load_factor: f64,
}

struct Entry<K, V> {
    hash: u64,
    key: K,
    value: V,
    psl: usize,
}

impl<K, V> Dominus<K, V> {
    pub fn new(capacity: usize, max_load_factor: f64) -> Self {
        let mut entries = Vec::new();
        entries.resize_with(
            capacity, 
            || RwLock::new(Option::<Entry<K, V>>::None)
        );
            
            Self {
                entries,
                size: AtomicUsize::new(0),
                capacity,
                max_load_factor,
            }
    }

    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    fn load_factor(&self) -> f64 {
        (self.size() as f64) / (self.capacity as f64)
    }

}

impl<K, V> Dominus<K, V> where 
    K: Hash + PartialEq,
    V: Copy,
{
    fn get_hash(&self, key: &K) -> u64 {
        let mut h = AHasher::default();
        key.hash(&mut h);
    
        h.finish()
    }

    pub fn get(&self, key: &K) -> Result<Option<V>> {
        let hash = self.get_hash(key);

        let mut entry_idx: usize = usize::try_from(hash)? % self.capacity;
        let mut psl = 0;

        loop {
            let current_entry = self.entries[entry_idx].read();
            match current_entry.as_ref() {
                Some(e) => {
                    if e.key == *key {
                        return Ok(Some(e.value));
                    }

                    if e.psl < psl {
                        return Ok(None);
                    }
                }

                None => {
                    return Ok(None);
                }
            }

            psl += 1;
            entry_idx = (entry_idx + 1) % self.capacity;
        }
    }

    // Return true if key already in table
    pub fn insert(&self, key: K, value: V) -> Result<Option<V>> {
        let hash = self.get_hash(&key);

        if self.load_factor() >= self.max_load_factor {
            return Err(anyhow!("Table is full (max load factor exceeded)"));
        }

        let mut entry_idx: usize = usize::try_from(hash)? % self.capacity;
        let mut entry_to_insert = Entry::new(hash, key, value, 0);
        let mut found = None;
        
        loop {
            let current_entry_r = self.entries[entry_idx].upgradable_read();

            match current_entry_r.as_ref() {
                Some(e) => {
                    if entry_to_insert.key == e.key {
                        found = Some(e.value);

                        let mut current_entry = RwLockUpgradableReadGuard::upgrade(current_entry_r);
                        *current_entry = Some(entry_to_insert);

                        break;
                    }

                    if entry_to_insert.psl > e.psl {
                        let mut current_entry = RwLockUpgradableReadGuard::upgrade(current_entry_r);
                        entry_to_insert = std::mem::replace(current_entry.deref_mut(), Some(entry_to_insert)).unwrap();
                    }
                    
                    entry_to_insert.psl += 1;
                }

                None => {
                    let mut current_entry = RwLockUpgradableReadGuard::upgrade(current_entry_r);
                    *current_entry = Some(entry_to_insert);

                    self.size.fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }

            entry_idx = (entry_idx + 1) % self.capacity;
        }

        Ok(found)
    }

    pub fn remove(&self, key: &K) -> Result<Option<V>> {
        let hash = self.get_hash(key);

        let mut entry_idx: usize = usize::try_from(hash)? % self.capacity;
        let mut current_entry = self.entries[entry_idx].upgradable_read();
        let mut psl = 0;
        

        let mut found = false;
        let mut old_value = None;

        while let Some(e) = current_entry.as_ref() {
            if e.key == *key {
                found = true;
                old_value = Some(e.value);
                break;
            }

            if e.psl < psl {
                break;
            }

            psl += 1;
            entry_idx = (entry_idx + 1) % self.capacity;
            current_entry = self.entries[entry_idx].upgradable_read();
        }

        if found {
            let mut prev_entry = RwLockUpgradableReadGuard::upgrade(current_entry);
            prev_entry.take();

            loop {
                entry_idx = (entry_idx + 1) % self.capacity;
                let next_entry_r = self.entries[entry_idx].upgradable_read();

                if let Some(e) = next_entry_r.as_ref()
                    && e.psl > 0
                {
                    let mut next_entry = RwLockUpgradableReadGuard::upgrade(next_entry_r);

                    let e = next_entry.as_mut().unwrap();
                    e.psl -= 1;

                    *prev_entry = next_entry.take();
                    prev_entry = next_entry;
                } 
                
                else {
                    break;
                }

            }
            
            self.size.fetch_sub(1, Ordering::Relaxed);
        }
        
        Ok(old_value)
    }
}
    

impl<K, V> Entry<K, V> {
    fn new(hash: u64, key: K, value: V, psl: usize) -> Self {
        Entry {
            hash,
            key,
            value,
            psl
        }
    }
}
