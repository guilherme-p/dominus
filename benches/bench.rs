use dominus::Dominus;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng;
use std::{collections::HashMap, sync::Arc, thread};


fn bench_1M_get(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let total_ops = 1_000_000;
    let key_range = 500_000;
    let table = Dominus::<i32, i32>::new((key_range as usize) * 2, 0.8);

    for _op in 0..total_ops {
        let (k, v) = (rng.gen_range(0..key_range), rng.gen());
        table.insert(k, v).unwrap();
    }

    c.bench_function("bench_1M_get", |b| b.iter(|| {
        for _op in 0..total_ops {
            let k = rng.gen_range(0..key_range);
            table.get(&k);
        }
    }));
}


fn bench_1M_get_concurrent(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let total_ops = 1_000_000;
    let key_range = 500_000;

    let dominus = Arc::new(Dominus::<i32, i32>::new((key_range as usize) * 2, 0.8));

    let n_threads_approx = thread::available_parallelism().unwrap().get();

    for _op in 0..total_ops {
        let (k, v): (i32, i32) = (rng.gen_range(0..key_range), rng.gen());
        dominus.insert(k, v).unwrap();
    }

    c.bench_function("bench_1M_get_concurrent", |b| b.iter(|| {
        let mut handles = Vec::new();

        for _t in 0..=n_threads_approx {
            let local = Arc::clone(&dominus);

            let handle = 
            thread::spawn(move || {
                let mut local_rng = rand::thread_rng();

                for _op in 0..(total_ops / n_threads_approx) {
                    let k = local_rng.gen_range(0..key_range);
                    let _got = local.get(&k);
                }
            }
            );

            handles.push(handle);
        }

        for h in handles.into_iter() {
            h.join().unwrap();
        }
    }));
}


fn bench_1M_insert_concurrent(c: &mut Criterion) {
    let total_ops = 1_000_000;
    let key_range = 500_000;

    let table = Arc::new(Dominus::<i32, i32>::new((key_range as usize) * 2, 0.8));

    let n_threads_approx = thread::available_parallelism().unwrap().get();

    c.bench_function("bench_1M_insert_concurrent", |b| b.iter(|| {
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
    }));
}


fn bench_1M_insert_mutex(c: &mut Criterion) {
    use parking_lot::Mutex;

    let total_ops = 100_000;
    let key_range: i32 = 50_000;

    let n_threads_approx = thread::available_parallelism().unwrap().get();

    let table = Arc::new(Mutex::new(HashMap::<i32, i32>::with_capacity((key_range as usize) * 2)));

    c.bench_function("bench_1M_insert_mutex", |b| b.iter(|| {
        let mut handles = Vec::new();

        for _t in 0..=n_threads_approx {
            let local = Arc::clone(&table);

            let handle = 
            thread::spawn(move || {
                let mut local_rng = rand::thread_rng();

                for _op in 0..(total_ops / n_threads_approx) {
                    let (k, v): (i32, i32) = (local_rng.gen_range(0..key_range), local_rng.gen());
                    let mut guard = local.lock();
                    guard.insert(k, v);
                }
            }
            );

            handles.push(handle);
        }

        for h in handles.into_iter() {
            h.join().unwrap();
        }
    }));

}

criterion_group!(benches, bench_1M_get, bench_1M_insert_mutex, bench_1M_get_concurrent, bench_1M_insert_concurrent);
criterion_main!(benches);
