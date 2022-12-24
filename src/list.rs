use crate::lazy::Lazy;
use std::ops::Index;
use std::rc::Rc;

fn create_evaluator<T: 'static, I: Iterator<Item = T> + 'static>(
    mut iter: I,
) -> Box<dyn FnOnce() -> LazyListInner<T>> {
    Box::new(move || match iter.next() {
        Some(item) => {
            let new_eval = Lazy::new(create_evaluator(iter));
            let rc = Rc::new(new_eval);
            LazyListInner::Evaluated(item, LazyList(rc))
        }
        None => LazyListInner::Terminated,
    })
}

fn create_cyclic_evaluator<T: 'static, F: FnMut(&LazyList<T>) -> Option<T> + 'static>(
    mut f: F,
    node: LazyList<T>,
) -> impl FnOnce() -> LazyListInner<T> {
    move || match f(&node) {
        Some(item) => {
            let inner_node = LazyList(Rc::new(Lazy::new(Box::new(create_cyclic_evaluator(
                f, node,
            )))));
            LazyListInner::Evaluated(item, inner_node)
        }
        None => LazyListInner::Terminated,
    }
}

type Thunk<T> = Lazy<LazyListInner<T>, Box<dyn FnOnce() -> LazyListInner<T>>>;
#[derive(Clone)]
pub struct LazyList<T>(Rc<Thunk<T>>);

enum LazyListInner<T> {
    Terminated,
    Evaluated(T, LazyList<T>),
}

impl<T: 'static> LazyList<T> {
    pub fn new() -> LazyList<T> {
        Self::emplace(LazyListInner::Terminated)
    }

    #[must_use]
    pub fn prepend(self, val: T) -> LazyList<T> {
        Self::emplace(LazyListInner::Evaluated(val, self))
    }

    pub fn new_cyclic<F: FnMut(&LazyList<T>) -> Option<T> + 'static>(f: F) -> LazyList<T> {
        let rc = Rc::new_cyclic(|w| {
            let w = w.clone();
            let b = Box::new(move || {
                let Some(s) = w.upgrade() else { unimplemented!() };
                let node = LazyList(s);
                create_cyclic_evaluator(f, node)()
            });
            Lazy::new(b)
        });
        LazyList(rc)
    }

    fn emplace(cell: LazyListInner<T>) -> Self {
        LazyList(Rc::new(Lazy::new(Box::new(move || cell))))
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.iter().nth(idx)
    }

    pub fn is_empty(&self) -> bool {
        matches!(Lazy::force(&*self.0), Some(LazyListInner::Terminated))
    }

    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn from_iter<I: IntoIterator<Item = T> + 'static>(iter: I) -> LazyList<T> {
        let iter = iter.into_iter();
        let contents = create_evaluator(iter);
        let rc: Rc<Lazy<_, Box<dyn FnOnce() -> _>>> = Rc::new(Lazy::new(contents));
        LazyList(rc)
    }

    pub fn iter(&self) -> Iter<T> {
        self.into_iter()
    }
}

impl<T: 'static> Default for LazyList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> Index<usize> for LazyList<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("Index out of range")
    }
}

impl<'a, T> IntoIterator for &'a LazyList<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        Iter(self)
    }
}

pub struct Iter<'a, T>(&'a LazyList<T>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        match Lazy::force(&*self.0 .0) {
            Some(LazyListInner::Evaluated(item, next)) => {
                self.0 = next;
                Some(item)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_count() {
        let list = LazyList::from_iter(0..10);
        assert_eq!(list.len(), 10);
    }

    #[test]
    fn iter_test() {
        let list = LazyList::from_iter(0..10);
        let mut k = 0;
        for &n in &list {
            assert_eq!(n, k);
            k += 1;
        }
        assert!(k == 10);
        assert_eq!(list.get(5), Some(&5));
    }

    #[test]
    fn singleton() {
        let list = LazyList::new();
        let list = list.prepend(0);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn empty() {
        let list: LazyList<u32> = LazyList::from_iter(std::iter::empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn primes() {
        let primes = LazyList::new_cyclic(|l| match l.len() {
            0 => Some(2),
            1 => Some(3),
            100 => None, // Make sure termination works for cyclic lists
            _ => {
                let mut n = l.iter().cloned().last().unwrap();
                'candidate: loop {
                    for factor in l.iter().cloned() {
                        if n % factor == 0 {
                            n += 2;
                            continue 'candidate;
                        }
                    }
                    break n;
                }
                .into()
            }
        });
        // Check the 100th prime
        assert_eq!(primes.iter().last().unwrap(), &541);
    }
}
