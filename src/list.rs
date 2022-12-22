use once_cell::unsync::Lazy;
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

#[derive(Clone)]
pub struct LazyList<T>(Rc<Lazy<LazyListInner<T>, Box<dyn FnOnce() -> LazyListInner<T>>>>);

enum LazyListInner<T> {
    Terminated,
    Evaluated(T, LazyList<T>),
}

impl<T: 'static> LazyList<T> {
    pub fn new() -> LazyList<T> {
        Self::emplace(LazyListInner::Terminated)
    }

    pub fn prepend(self, val: T) -> LazyList<T> {
        Self::emplace(LazyListInner::Evaluated(val, self))
    }

    fn emplace(cell: LazyListInner<T>) -> Self {
        LazyList(Rc::new(Lazy::new(Box::new(move || cell))))
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        match &**self.0 {
            LazyListInner::Terminated => None,
            LazyListInner::Evaluated(item, next) => {
                if idx == 0 {
                    Some(item)
                } else {
                    next.get(idx - 1)
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.accum(0)
    }

    fn accum(&self, count: usize) -> usize {
        match &**self.0 {
            LazyListInner::Terminated => count,
            LazyListInner::Evaluated(_, next) => next.accum(count + 1),
        }
    }

    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> LazyList<T>
    where
        I: 'static,
    {
        let iter = iter.into_iter();
        let contents = create_evaluator(iter);
        let rc: Rc<Lazy<_, Box<dyn FnOnce() -> _>>> = Rc::new(Lazy::new(Box::new(contents)));
        LazyList(rc)
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        self.into_iter()
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
        match &**self.0 .0 {
            LazyListInner::Evaluated(item, next) => {
                self.0 = next;
                Some(item)
            }
            LazyListInner::Terminated => None,
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
    }

    #[test]
    fn blank() {
        let list = LazyList::new();
        let list = list.prepend(0);
        assert_eq!(list.len(), 1);
    }
}
