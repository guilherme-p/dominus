#![feature(test)]

use parking_lot::RwLock;
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
    pub fn get(&self, key: K) -> Option<V> {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);

        let hash = h.finish();

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.capacity;
        let mut psl = 0;

        loop {
            let current_entry = self.entries[entry_idx].read();
            match (*current_entry).as_ref() {
                Some(e) => {
                    if e.key == key {
                        return Some(e.value);
                    }

                    if e.psl >= psl {
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
        let mut h = DefaultHasher::new();
        key.hash(&mut h);

        let hash = h.finish();

        if self.load_factor() >= self.max_load_factor {
            return Err("Table is full (max load factor exceeded)".into());
        }

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.capacity;
        let mut entry_to_insert = Entry::new(hash, key, value, 0);
        let mut found = false;

        loop {
            let current_entry = self.entries[entry_idx].read();
            match (*current_entry).as_ref() {
                Some(e) => {
                    if entry_to_insert.key == e.key {
                        let mut w = self.entries[entry_idx].write();
                        *w = Some(entry_to_insert);

                        found = true;
                        break;
                    }

                    if entry_to_insert.psl > e.psl {
                        let mut w = self.entries[entry_idx].write();
                        entry_to_insert = std::mem::replace(w.deref_mut(), Some(entry_to_insert)).unwrap();
                    } else {
                        entry_to_insert.psl += 1;
                    }
                }

                None => {
                    let mut w = self.entries[entry_idx].write();
                    *w = Some(entry_to_insert);

                    break;
                }
            }

            entry_idx = (entry_idx + 1) % self.capacity;
        }

        self.size.fetch_add(1, Ordering::Relaxed);
        Ok(found)
    }

    pub fn remove(&self, key: K) -> bool {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);

        let hash = h.finish();

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.capacity;
        let mut current_entry = self.entries[entry_idx].read();
        let mut psl = 0;
        

        let mut found = false;

        loop {
            match (*current_entry).as_ref() {
                Some(e) => {
                    if e.key == key {
                        found = true;
                        break;
                    }

                    if e.psl >= psl {
                        break;
                    }
                }

                None => {
                    break;
                }
            }

            psl += 1;
            entry_idx = (entry_idx + 1) % self.capacity;
            current_entry = self.entries[entry_idx].read();
        }

        if found {
            let mut prev_entry = self.entries[entry_idx].write();
            prev_entry.take();

            loop {
                entry_idx = (entry_idx + 1) % self.capacity;

                let next_entry_r = self.entries[entry_idx].read();

                if next_entry_r.is_some() {
                    let mut next_entry = self.entries[entry_idx].write();
                    let mut e = (*next_entry).as_mut().unwrap();
                    e.psl -= 1;

                    *prev_entry = next_entry.take();
                    prev_entry = next_entry;
                } else {
                    break;
                }

            }
            
            self.size.fetch_sub(1, Ordering::Relaxed);
        }
        
        found
    }
}
    

impl<K, V> Entry<K, V> {
    fn new(hash: u64, key: K, value: V, psl: usize,) -> Self {
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
    use test::Bencher;
    
    #[bench]
    unsafe fn bench_70_30(b: &mut Bencher) {
        use std::thread::available_parallelism;
        use rand::Rng;
        use rand::seq::SliceRandom;

        let n_threads_approx = available_parallelism().unwrap().get();
        let mut rng = rand::thread_rng();
        let total_ops = 5_000_000;

        let mut table = Arc::new(Dominus::<i32, i32>::new(1_000_000, 0.8));

        b.iter(|| {
            let mut handles = Vec::new();

            for t in 0..=n_threads_approx {
                let local = Arc::clone(&table);

                let handle = 
                    thread::spawn(move || {
                        let mut local_rng = rand::thread_rng();
                        let mut entries: Vec<(i32, i32)> = Vec::new();

                        for op in 0..total_ops / n_threads_approx {
                            if local_rng.gen::<f64>() < 0.3 {
                                let (k, v) = (local_rng.gen(), local_rng.gen());
                                local.insert(k, v);
                                entries.push((k, v));
                            } else {
                                let o = entries.choose(&mut local_rng);
                                if o.is_some() {
                                    let (k, _) = o.unwrap();
                                    local.get(*k);
                                }
                            }
                        }
                    }
                );

                handles.push(handle);
            }

            for h in handles.into_iter() {
                h.join().unwrap();
            }
        });
    }
}
