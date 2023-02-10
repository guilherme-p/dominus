use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    fn new(num_buckets: usize, bucket_capacity: usize) -> Self {
        /* let mut empty_entries = Vec::new();
        empty_entries.resize_with(bucket_capacity, || Option::<Entry<K, V>>::None); */
        
        let mut buckets = Vec::new();
        buckets.resize_with(num_buckets, || RwLock::new(Bucket::new(
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
