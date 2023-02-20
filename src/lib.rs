#![feature(test)]
#![feature(let_chains)]

use parking_lot::RwLock;
use parking_lot::RwLockUpgradableReadGuard;
use std::error::Error;
use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
        let mut h = DefaultHasher::new();
        key.hash(&mut h);
    
        h.finish()
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let hash = self.get_hash(key);

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.capacity;
        let mut psl = 0;

        loop {
            let current_entry = self.entries[entry_idx].read();
            match (*current_entry).as_ref() {
                Some(e) => {
                    if e.key == *key {
                        return Some(e.value);
                    }

                    if e.psl < psl {
                        return None;
                    }
                }

                None => {
                    return None;
                }
            }

            psl += 1;
            entry_idx = (entry_idx + 1) % self.capacity;
        }
    }

    // Return true if key already in table
    pub fn insert(&self, key: K, value: V) -> Result<bool, Box<dyn Error>> {
        let hash = self.get_hash(&key);

        if self.load_factor() >= self.max_load_factor {
            return Err("Table is full (max load factor exceeded)".into());
        }

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.capacity;
        let mut entry_to_insert = Entry::new(hash, key, value, 0);
        let mut found = false;
        
        loop {
            let current_entry_r = self.entries[entry_idx].upgradable_read();

            match (*current_entry_r).as_ref() {
                Some(e) => {
                    if entry_to_insert.key == e.key {
                        let mut current_entry = RwLockUpgradableReadGuard::upgrade(current_entry_r);
                        *current_entry = Some(entry_to_insert);

                        found = true;
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

    pub fn remove(&self, key: &K) -> bool {
        let hash = self.get_hash(key);

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.capacity;
        let mut current_entry = self.entries[entry_idx].upgradable_read();
        let mut psl = 0;
        

        let mut found = false;

        loop {
            match (*current_entry).as_ref() {
                Some(e) => {
                    if e.key == *key {
                        found = true;
                        break;
                    }

                    if e.psl < psl {
                        break;
                    }
                }

                None => {
                    break;
                }
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

                if let Some(e) = (*next_entry_r).as_ref()
                    && e.psl > 0
                {
                    let mut next_entry = RwLockUpgradableReadGuard::upgrade(next_entry_r);

                    let mut e = (*next_entry).as_mut().unwrap();
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
        
        found
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

extern crate test;
#[cfg(test)]
mod tests {
    use std::thread;
    use super::*;
    use rand::Rng;
    use rand::seq::SliceRandom;
    use rand::prelude::IteratorRandom;
    use std::collections::HashMap;

    #[test]
    fn test_insert_get() {
        let mut rng = rand::thread_rng();

        let total_ops = 50_000;
        let mut table = Dominus::<i32, i32>::new(100_000, 0.8);
        let mut entries: HashMap<i32, i32> = HashMap::new();

        for _op in 0..total_ops {
            let (k, v) = (rng.gen(), rng.gen());
            table.insert(k, v).unwrap();
            entries.insert(k, v);
        }

        let mut entries = Vec::from_iter(entries);

        for _e in 0..entries.len() {
            let (i, o) = (&mut entries).into_iter().enumerate().choose(&mut rng).unwrap();
            
            let (k, v) = *o;
            let got = table.get(&k).unwrap();
            assert_eq!(got, v);
            
            entries.remove(i);
        }
    }

    #[test]
    fn test_insert_load_factor() {
        let mut rng = rand::thread_rng();

        let total_ops = 80_000;
        let mut table = Dominus::<i32, i32>::new(100_000, 0.8);
        let mut entries: HashMap<i32, i32> = HashMap::new();

        for _op in 0..total_ops {
            let (mut k, v): (i32, i32) = (rng.gen(), rng.gen());
            while entries.contains_key(&k) {
                k = rng.gen();
            }

            table.insert(k, v).unwrap();
            entries.insert(k, v);
        }
        
        let (mut k, v): (i32, i32) = (rng.gen(), rng.gen());
        while entries.contains_key(&k) {
            k = rng.gen();
        }

        let res = table.insert(k, v);
        assert!(res.is_err());
    }

    #[test]
    fn test_insert_remove() {
        let mut rng = rand::thread_rng();

        let total_ops = 50_000;
        let mut table = Dominus::<i32, i32>::new(100_000, 0.8);
        let mut entries: HashMap<i32, i32> = HashMap::new();

        for _op in 0..total_ops {
            let (k, v) = (rng.gen(), rng.gen());
            table.insert(k, v).unwrap();
            entries.insert(k, v);
        }

        let mut entries = Vec::from_iter(entries);

        for _e in 0..entries.len() {
            let (i, o) = (&mut entries).into_iter().enumerate().choose(&mut rng).unwrap();
            
            let (k, v) = *o;
            let removed = table.remove(&k);
            assert!(removed);
            
            entries.remove(i);
        }
    }
    
    #[test]
    fn test_70_30() {
        let n_threads_approx = thread::available_parallelism().unwrap().get();
        let mut rng = rand::thread_rng();
        let total_ops = 500_000;

        let mut table = Arc::new(Dominus::<i32, i32>::new(1_000_000, 0.8));

        let mut handles = Vec::new();

        for t in 0..=n_threads_approx {
            let local = Arc::clone(&table);

            let handle = 
                thread::spawn(move || {
                    let mut local_rng = rand::thread_rng();
                    let mut entries: Vec<(i32, i32)> = Vec::new();

                    for op in 0..(total_ops / n_threads_approx) {
                        if local_rng.gen::<f64>() < 0.3 {
                            let (k, v) = (local_rng.gen(), local_rng.gen());
                            local.insert(k, v).unwrap();
                            entries.push((k, v));
                        } 
                        
                        else if entries.len() > 0 {
                            let o = entries.choose(&mut local_rng);
                            let (k, _) = o.unwrap();
                            let res = local.get(k);

                            assert!(res.is_some());
                        }
                    }
                }
            );

            handles.push(handle);
        }

        for h in handles.into_iter() {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_33_contention() {
        let mut rng = rand::thread_rng();
        let total_ops = 500_000;
        let key_range = 1000;
        
        let mut table = Arc::new(Dominus::<i32, i32>::new(10_000, 0.8));
        
        let n_threads_approx = thread::available_parallelism().unwrap().get();
        let mut handles = Vec::new();

        for _t in 0..=n_threads_approx {
            let local = Arc::clone(&table);

            let handle = 
                thread::spawn(move || {
                    let mut local_rng = rand::thread_rng();

                    for _op in 0..(total_ops / n_threads_approx) {
                        let p = local_rng.gen::<f64>();

                        if p < 0.33 {
                            let (k, v): (i32, i32) = (local_rng.gen_range(0..key_range), local_rng.gen());
                            local.insert(k, v).unwrap();
                        } 
                        
                        else if p < 0.66 {
                            let k = local_rng.gen_range(0..key_range);
                            local.get(&k);
                        }

                        else {
                            let k = local_rng.gen_range(0..key_range);
                            local.remove(&k);
                        }
                    }
                }
            );

            handles.push(handle);
        }

        for h in handles.into_iter() {
            h.join().unwrap();
        }
    }
}
