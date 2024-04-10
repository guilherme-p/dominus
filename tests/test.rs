use dominus::*;
use dashmap::DashMap;
use std::thread::{self, JoinHandle};
use rand::Rng;
use rand::seq::SliceRandom;
use rand::prelude::IteratorRandom;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[test]
fn test_insert_get() {
    let mut rng = rand::thread_rng();

    let total_ops = 50_000;
    let dominus = Dominus::<i32, i32>::new(100_000, 0.8);
    let mut entries: HashMap<i32, i32> = HashMap::new();

    for _op in 0..total_ops {
        let (k, v) = (rng.gen(), rng.gen());
        dominus.insert(k, v).unwrap();
        entries.insert(k, v);
    }

    let mut entries = Vec::from_iter(entries);

    for _e in 0..entries.len() {
        let (i, o) = entries.iter().enumerate().choose(&mut rng).unwrap();

        let (k, v) = *o;
        let got = dominus.get(&k).unwrap().unwrap();
        assert_eq!(got, v);

        entries.remove(i);
    }
}

#[test]
fn test_insert_load_factor() {
    let mut rng = rand::thread_rng();

    let total_ops = 80_000;
    let dominus = Dominus::<i32, i32>::new(100_000, 0.8);
    let mut entries: HashMap<i32, i32> = HashMap::new();

    for _op in 0..total_ops {
        let (mut k, v): (i32, i32) = (rng.gen(), rng.gen());
        while entries.contains_key(&k) {
            k = rng.gen();
        }

        dominus.insert(k, v).unwrap();
        entries.insert(k, v);
    }

    let (mut k, v): (i32, i32) = (rng.gen(), rng.gen());
    while entries.contains_key(&k) {
        k = rng.gen();
    }

    let res = dominus.insert(k, v);
    assert!(res.is_err());
}

#[test]
fn test_insert_remove() {
    let mut rng = rand::thread_rng();

    let total_ops = 50_000;
    let dominus = Dominus::<i32, i32>::new(100_000, 0.8);
    let mut entries: HashMap<i32, i32> = HashMap::new();

    for _op in 0..total_ops {
        let (k, v) = (rng.gen(), rng.gen());
        dominus.insert(k, v).unwrap();
        entries.insert(k, v);
    }

    for _e in 0..entries.len() {
        let o = entries.iter().choose(&mut rng);

        let o = o.map(|(a, b)| (*a, *b));
        let (k, v) = o.unwrap();

        let removed = dominus.remove(&k).unwrap().unwrap();
        assert_eq!(removed, v);

        entries.remove(&k);
    }
}

#[test]
fn test_concurrent_insert() {
    let n_threads_approx = thread::available_parallelism().unwrap().get();
    let total_ops = 100_000;

    let dominus = Arc::new(Dominus::<i32, i32>::new(200_000, 0.8));
    let mut entries = HashMap::<i32, i32>::new();

    let mut rng = rand::thread_rng();

    for _op in 0..total_ops {
        let (k, v) = (rng.gen(), rng.gen());
        entries.insert(k, v);
    }

    let entries_vec: Vec<(i32, i32)> = entries.iter().into_iter().map(|(a, b)| (*a, *b)).collect();
    let entries_chunks = entries_vec.chunks(entries_vec.len() / n_threads_approx).map(|c| c.to_owned());

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    for chunk in entries_chunks {
        let dominus_clone = Arc::clone(&dominus);

        let handle = 
        thread::spawn(move || {
            for (k, v) in chunk {
                dominus_clone.insert(k, v).unwrap();
            }
        });

        handles.push(handle);
    }

    for h in handles.into_iter() {
        h.join().unwrap();
    }

    for (k, v) in entries {
        let res = dominus.get(&k).unwrap().unwrap();
        assert_eq!(res, v);
    }
}


// #[test]
// fn test_33_contention() {
//     let total_ops = 100_000;
//     let key_range = 1000;
//
//     let size = (key_range as usize) * 2;
//
//     let dominus = Arc::new(Dominus::<i32, i32>::new(size, 0.8));
//     let dashmap = Arc::new(DashMap::<i32, i32>::with_capacity(size));
//
//     let n_threads_approx = thread::available_parallelism().unwrap().get();
//     let mut handles = Vec::new();
//
//     for _t in 0..=n_threads_approx {
//         let dominus_clone = Arc::clone(&dominus);
//         let dashmap_clone = Arc::clone(&dashmap);
//
//         let handle = 
//         thread::spawn(move || {
//             let mut local_rng = rand::thread_rng();
//
//             for _op in 0..(total_ops / n_threads_approx) {
//                 let p = local_rng.gen::<f64>();
//
//                 if p < 0.33 {
//                     let (k, v): (i32, i32) = (local_rng.gen_range(0..key_range), local_rng.gen());
//                     let t1 = dominus_clone.insert(k, v).unwrap();
//                     let t2 = dashmap.insert(k, v);
//                     assert_eq!(t1, t2);
//                 } 
//
//                 else if p < 0.66 {
//                     let o = entries_clone_guard.iter().choose(&mut local_rng);
//                     let o = o.map(|(a, b)| (*a, *b));
//                     let (k, v) = o.unwrap();
//                     let res = dominus_clone.get(&k).unwrap().unwrap();
//                     assert_eq!(res, v);
//                 }
//
//                 else {
//                     let o = entries_clone_guard.iter().choose(&mut local_rng);
//                     let o = o.map(|(a, b)| (*a, *b));
//                     let (k, v) = o.unwrap();
//
//                     let removed = dominus_clone.remove(&k).unwrap().unwrap();
//                     assert_eq!(removed, v);
//                     dashmap_clone.lock().unwrap().remove(&k);
//                 }
//             }
//         }
//         );
//
//         handles.push(handle);
//     }
//
//     for h in handles.into_iter() {
//         h.join().unwrap();
//     }
// }
