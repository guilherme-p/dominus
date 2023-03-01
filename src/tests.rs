extern crate test;

#[cfg(test)]
mod tests {
    use crate::dominus::Dominus;
    use std::thread;
    use rand::Rng;
    use rand::seq::SliceRandom;
    use rand::prelude::IteratorRandom;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_insert_get() {
        let mut rng = rand::thread_rng();

        let total_ops = 50_000;
        let table = Dominus::<i32, i32>::new(100_000, 0.8);
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
        let table = Dominus::<i32, i32>::new(100_000, 0.8);
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
        let table = Dominus::<i32, i32>::new(100_000, 0.8);
        let mut entries: HashMap<i32, i32> = HashMap::new();

        for _op in 0..total_ops {
            let (k, v) = (rng.gen(), rng.gen());
            table.insert(k, v).unwrap();
            entries.insert(k, v);
        }

        let mut entries = Vec::from_iter(entries);

        for _e in 0..entries.len() {
            let (i, o) = (&mut entries).into_iter().enumerate().choose(&mut rng).unwrap();
            
            let (k, _v) = *o;
            let removed = table.remove(&k);
            assert!(removed);
            
            entries.remove(i);
        }
    }
    
    #[test]
    fn test_70_30() {
        let n_threads_approx = thread::available_parallelism().unwrap().get();
        let total_ops = 500_000;

        let table = Arc::new(Dominus::<i32, i32>::new(1_000_000, 0.8));

        let mut handles = Vec::new();

        for _t in 0..=n_threads_approx {
            let local = Arc::clone(&table);

            let handle = 
                thread::spawn(move || {
                    let mut local_rng = rand::thread_rng();
                    let mut entries: Vec<(i32, i32)> = Vec::new();

                    for _op in 0..(total_ops / n_threads_approx) {
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
        let total_ops = 500_000;
        let key_range = 1000;
        
        let table = Arc::new(Dominus::<i32, i32>::new((key_range as usize) * 2, 0.8));
        
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