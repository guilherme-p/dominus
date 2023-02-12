use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Dominus<K, V> {
    buckets: Vec<RwLock<Bucket<K, V>>>,
    size: Arc<AtomicUsize>,
    num_buckets: usize,
    bucket_capacity: usize,
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
    pub fn new(num_buckets: usize, bucket_capacity: usize) -> Self {
        /* let mut empty_entries = Vec::new();
        empty_entries.resize_with(bucket_capacity, || Option::<Entry<K, V>>::None); */
        
        let mut buckets = Vec::new();
        buckets.resize_with(
            num_buckets, 
            || RwLock::new(
                Bucket::new(
                    std::iter::repeat_with(|| Option::<Entry<K, V>>::None).take(bucket_capacity).collect::<Vec<Option<Entry<K, V>>>>(), // Using repeat_with so we don't need a Clone trait bound
                            bucket_capacity))
        );
            
            Self {
                buckets,
                size: Arc::new(AtomicUsize::new(0)),
                num_buckets,
                bucket_capacity,
            }
    }

    pub fn get_load_capacity(&self) -> f32 {
        self.size.load(Ordering::Relaxed) as f32 / ((self.num_buckets * self.bucket_capacity) as f32)
    }
}

impl<K, V> Dominus<K, V> where 
    K: Hash,
{
    pub fn insert(&mut self, key: K, value: V) {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);

        let hash = h.finish();
        let bucket_idx: usize = usize::try_from(hash).unwrap() % self.num_buckets;
        let bucket = &mut self.buckets[bucket_idx].write();

        let mut entry_idx: usize = usize::try_from(hash).unwrap() % self.bucket_capacity;
        let mut entry_to_insert = Entry::new(hash, key, value, 0);

        loop {
            let current_entry = &bucket.entries[entry_idx];
            match current_entry {
                Some(e) => {
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
