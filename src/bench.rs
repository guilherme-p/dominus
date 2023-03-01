#[allow(non_snake_case)]

#[cfg(test)]
mod bench {
    use crate::dominus::Dominus;
    use std::thread;
    use rand::Rng;
    use std::collections::HashMap;
    use std::sync::Arc;
    use test::Bencher;
    
    #[bench]
    fn bench_1M_get(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let total_ops = 1_000_000;
        let key_range = 500_000;
        let table = Dominus::<i32, i32>::new((key_range as usize) * 2, 0.8);

        for _op in 0..total_ops {
            let (k, v) = (rng.gen_range(0..key_range), rng.gen());
            table.insert(k, v).unwrap();
        }

        b.iter(|| {
            for _op in 0..total_ops {
                let k = rng.gen_range(0..key_range);
                table.get(&k);
            }
        });
    }

    #[bench]
    fn bench_1M_get_concurrent(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let total_ops = 1_000_000;
        let key_range = 500_000;
        
        let table = Arc::new(Dominus::<i32, i32>::new((key_range as usize) * 2, 0.8));
        
        let n_threads_approx = thread::available_parallelism().unwrap().get();
        
        for _op in 0..total_ops {
            let (k, v): (i32, i32) = (rng.gen_range(0..key_range), rng.gen());
            table.insert(k, v).unwrap();
        }
        
        b.iter(|| {
            let mut handles = Vec::new();

            for _t in 0..=n_threads_approx {
                let local = Arc::clone(&table);
    
                let handle = 
                    thread::spawn(move || {
                        let mut local_rng = rand::thread_rng();
    
                        for _op in 0..(total_ops / n_threads_approx) {
                            let k = local_rng.gen_range(0..key_range);
                            local.get(&k);
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

    #[bench]
    fn bench_1M_insert_concurrent(b: &mut Bencher) {
        let total_ops = 1_000_000;
        let key_range = 500_000;
        
        let table = Arc::new(Dominus::<i32, i32>::new((key_range as usize) * 2, 0.8));
        
        let n_threads_approx = thread::available_parallelism().unwrap().get();
        
        b.iter(|| {
            let mut handles = Vec::new();

            for _t in 0..=n_threads_approx {
                let local = Arc::clone(&table);
    
                let handle = 
                    thread::spawn(move || {
                        let mut local_rng = rand::thread_rng();
    
                        for _op in 0..(total_ops / n_threads_approx) {
                            let (k, v): (i32, i32) = (local_rng.gen_range(0..key_range), local_rng.gen());
                            local.insert(k, v).unwrap();
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

    #[bench]
    fn bench_1M_insert_mutex(b: &mut Bencher) {
        use parking_lot::Mutex;

        let total_ops = 1_000_000;
        let key_range: i32 = 500_000;
        let mut rng = rand::thread_rng();
        
        let table = Mutex::new(HashMap::<i32, i32>::with_capacity((key_range as usize) * 2));
        
        b.iter(|| {
            for _op in 0..total_ops {
                let (k, v): (i32, i32) = (rng.gen_range(0..key_range), rng.gen());
                let mut guard = table.lock();
                guard.insert(k, v);
            }
        });
    }
}