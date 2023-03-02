I made this simple, concurrent Robin Hood hash table in order to practice Rust. It uses parking_lot's RwLock for each bucket, and ahash for faster hashing.

## Concurrency reasoning

> **WARNING**: Reasoning about concurrency is very hard and error-prone, you should NOT trust any of this. I have NOT dedicated a lot of time to this project, and I'm FAR from an expert on this. This logic could be completely WRONG.

A bucket is only exclusively locked when being written to or deleted. When being deleted, to apply backward shifting we lock both buckets for the swap.

There is no data race on deletion due to the use of upgradeable reads (which are mutually exclusive with writes). By using them, we don't need to exclusively lock until we have found the node/read the next node and know we need to backward shift, allowing reads in the meantime and avoiding expensive locks.

There is no data race on insertion for the same reason. If while inserting, a thread removes some node behind us, it will backward shift us into the correct position. If another thread tries to get the inserted key after the remove but before the backward shift has finalized, it will block because the backward shifting thread is holding exclusive locks in its path.

If while inserting, a thread removes some node in front of us, the deleting thread is always holding exclusive locks to any nodes that may cause an inconsistent state/violate Robin Hood invariants.

## References:

[1] https://programming.guide/robin-hood-hashing.html

[2] https://docs.rs/parking_lot/latest/parking_lot/type.RwLock.html

[3] https://docs.rs/lock_api/0.4.7/lock_api/struct.RwLock.html#method.upgradable_read