use std::cell::RefCell;
use std::ops::Index;
use std::rc::Rc;

#[derive(Clone)]
pub struct LazyList<T>(Rc<RefCell<LazyListInner<T>>>);

enum LazyListInner<T> {
    Terminated,
    Thunk(Box<dyn Iterator<Item = T>>),
    Evaluated(T, LazyList<T>),
}

impl<T> LazyList<T> {
    pub fn new() -> LazyList<T> {
        Self::emplace(LazyListInner::Terminated)
    }

    pub fn prepend(self, val: T) -> LazyList<T> {
        Self::emplace(LazyListInner::Evaluated(val, self))
    }

    fn emplace(cell: LazyListInner<T>) -> Self {
        LazyList(Rc::new(RefCell::new(cell)))
    }

    fn expand(&self) {
        let mut inner = self.0.borrow_mut();

        let payload = std::mem::replace(&mut *inner, LazyListInner::Terminated);
        *inner = if let LazyListInner::Thunk(mut thunk) = payload {
            match thunk.next() {
                Some(item) => {
                    LazyListInner::Evaluated(item, Self::emplace(LazyListInner::Thunk(thunk)))
                }
                None => LazyListInner::Terminated,
            }
        } else {
            payload
        };
    }

    pub fn len(&self) -> usize {
        self.accum(0)
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.expand();
        match &*self.0.borrow() {
            LazyListInner::Evaluated(item, next) => {
                if idx == 0 {
                    let ptr = item as *const T;
                    unsafe { Some(&*ptr) }
                } else {
                    let ptr = next as *const LazyList<T>;
                    unsafe { (&*ptr).get(idx - 1) }
                }
            }
            LazyListInner::Terminated => None,
            LazyListInner::Thunk(_) => unreachable!(),
        }
    }

    fn accum(&self, count: usize) -> usize {
        self.expand();
        match &*self.0.borrow() {
            LazyListInner::Terminated => count,
            LazyListInner::Evaluated(_, next) => next.accum(count + 1),
            LazyListInner::Thunk(_) => unreachable!(),
        }
    }

    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self
    where
        I: 'static,
    {
        let iter = iter.into_iter();
        let contents = LazyListInner::Thunk(Box::new(iter));
        LazyList::emplace(contents)
    }
}

impl<T> Index<usize> for LazyList<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("Index out of range")
    }
}
/*
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
        self.0.expand();
        match &*self.0 .0.borrow() {
            LazyListInner::Evaluated(item, next) => {
                self.0 = next;
                Some(&*item)
            }
            LazyListInner::Thunk(_) => unreachable!(),
            LazyListInner::Terminated => None,
        }
    }
} */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_count() {
        let list = LazyList::from_iter(0..10);
        assert_eq!(list.len(), 10);
    }

    #[test]
    fn blank() {
        let list = LazyList::new();
        let list = list.prepend(0);
        assert_eq!(list.len(), 1);
    }
}
