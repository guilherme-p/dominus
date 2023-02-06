pub struct Dominus<T> {
    table: Box<Vec<T>>,
    size: usize,
}

struct Entry<T> {
    key: u64,
    value: T,
    psl: usize,
}

impl<T> Dominus<T> {
    fn new(size: usize) -> Self {
        Self {
            table: Box::new(Vec::with_capacity(size)),
            size
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
