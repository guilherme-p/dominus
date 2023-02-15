use parking_lot::RwLock;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Dominus<K, V> {
    buckets: Vec<RwLock<Bucket<K, V>>>,
    size: AtomicUsize,
    num_buckets: usize,
    bucket_capacity: usize,
    max_bucket_load_factor: f64,
}

struct Bucket<K, V> {
    entries: Vec<Option<Entry<K, V>>>,
    size: usize,
    capacity: usize,
}

struct Entry<K, V> {
    hash: u64,
    key: K,
    value: V,
    psl: usize,
}


impl<K, V> Dominus<K, V> {
    pub fn new(num_buckets: usize, bucket_capacity: usize, max_bucket_load_factor: f64) -> Self {
        let mut buckets = Vec::new();
        buckets.resize_with(
            num_buckets, 
            || RwLock::new(
                Bucket::new(
                    std::iter::repeat_with(|| Option::<Entry<K, V>>::None).take(bucket_capacity).collect::<Vec<Option<Entry<K, V>>>>(), // Using repeat_with so we don't need a Copy trait bound
                            bucket_capacity))
        );
            
            Self {
                buckets,
                size: AtomicUsize::new(0),
                num_buckets,
                bucket_capacity,
                max_bucket_load_factor,
            }
    }

    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
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
        let bucket_idx: usize = usize::try_from(hash).unwrap() % self.num_buckets;
        let bucket = &self.buckets[bucket_idx].read();

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.bucket_capacity;
        let mut psl = 0;

        loop {
            let current_entry = &bucket.entries[entry_idx];
            match current_entry {
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
            entry_idx = (entry_idx + 1) % self.bucket_capacity;
        }
    }

    // Return true if key already in table
    pub fn insert(&mut self, key: K, value: V) -> Result<bool, Box<dyn Error>> {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);

        let hash = h.finish();
        let bucket_idx: usize = usize::try_from(hash).unwrap() % self.num_buckets;
        let bucket = &mut self.buckets[bucket_idx].write();

        if bucket.load_factor() >= self.max_bucket_load_factor {
            return Err("Bucket is full (max load factor exceeded)".into());
        }

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.bucket_capacity;
        let mut entry_to_insert = Entry::new(hash, key, value, 0);
        let mut found = false;

        loop {
            let current_entry = &bucket.entries[entry_idx];
            match current_entry {
                Some(e) => {
                    if entry_to_insert.key == e.key {
                        bucket.entries[entry_idx] = Some(entry_to_insert);
                        found = true;
                        break;
                    }

                    if entry_to_insert.psl > e.psl {
                        entry_to_insert = std::mem::replace(&mut bucket.entries[entry_idx], Some(entry_to_insert)).unwrap();
                    } else {
                        entry_to_insert.psl += 1;
                    }
                }

                None => {
                    bucket.entries[entry_idx] = Some(entry_to_insert);
                    break;
                }
            }

            entry_idx = (entry_idx + 1) % self.bucket_capacity;
        }

        bucket.size += 1;
        self.size.fetch_add(1, Ordering::Relaxed);

        Ok(found)
    }

    pub fn remove(&mut self, key: K) -> bool {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);

        let hash = h.finish();
        let bucket_idx: usize = usize::try_from(hash).unwrap() % self.num_buckets;
        let bucket = &mut self.buckets[bucket_idx].write();

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.bucket_capacity;
        let mut current_entry = &bucket.entries[entry_idx];
        let mut psl = 0;
        

        let mut found = false;

        loop {
            match current_entry {
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
            entry_idx = (entry_idx + 1) % self.bucket_capacity;
            current_entry = &bucket.entries[entry_idx];
        }

        if found {
            bucket.entries[entry_idx].take();

            loop {
                let prev_idx = entry_idx;
                entry_idx = (entry_idx + 1) % self.bucket_capacity;
                let current_entry = &mut bucket.entries[entry_idx];

                match current_entry {
                    Some(e) => {
                        e.psl -= 1;
                        bucket.entries.swap(prev_idx, entry_idx);
                    }
    
                    None => {
                        break;
                    }
                }
            }
        }

        found
    }
}
    
    
impl<K, V> Bucket<K, V> {
    fn new(entries: Vec<Option<Entry<K, V>>>, capacity: usize) -> Self {
        Self {
            entries,
            size: 0,
            capacity,
        }
    }

    fn load_factor(&self) -> f64 {
        (self.size as f64) / (self.capacity as f64)
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
    
    
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
