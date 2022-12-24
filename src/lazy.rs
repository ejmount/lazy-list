use once_cell::sync::OnceCell;
use std::cell::Cell;
use std::fmt;
use std::panic::RefUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Lazy<T, F = fn() -> T> {
    cell: OnceCell<T>,
    init: Cell<Option<F>>,
    in_progress: AtomicBool,
}

impl<T: fmt::Debug, F> fmt::Debug for Lazy<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Lazy")
            .field("cell", &self.cell)
            .field("init", &"..")
            .finish()
    }
}

// We never create a `&F` from a `&Lazy<T, F>` so it is fine to not impl
// `Sync` for `F`. We do create a `&mut Option<F>` in `force`, but this is
// properly synchronized, so it only happens once so it also does not
// contribute to this impl.
unsafe impl<T, F: Send> Sync for Lazy<T, F> where OnceCell<T>: Sync {}
// auto-derived `Send` impl is OK.

impl<T, F: RefUnwindSafe> RefUnwindSafe for Lazy<T, F> where OnceCell<T>: RefUnwindSafe {}

impl<T, F> Lazy<T, F> {
    /// Creates a new lazy value with the given initializing
    /// function.
    pub const fn new(f: F) -> Lazy<T, F> {
        Lazy {
            cell: OnceCell::new(),
            init: Cell::new(Some(f)),
            in_progress: AtomicBool::new(false),
        }
    }
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    /// Forces the evaluation of this lazy value and
    /// returns a reference to the result. This is equivalent
    /// to the `Deref` impl, but is explicit.
    ///
    /// # Example
    /// ```
    /// use once_cell::sync::Lazy;
    ///
    /// let lazy = Lazy::new(|| 92);
    ///
    /// assert_eq!(Lazy::force(&lazy), &92);
    /// assert_eq!(&*lazy, &92);
    /// ```
    pub fn force(this: &Lazy<T, F>) -> Option<&T> {
        if this.in_progress.load(Ordering::Acquire) {
            return None;
        }
        this.cell
            .get_or_init(|| match this.init.take() {
                Some(f) => {
                    this.in_progress.store(true, Ordering::Release);
                    let _c = Canary(&this.in_progress);
                    f()
                }
                None => panic!("Lazy instance has previously been poisoned"),
            })
            .into()
    }
}

impl<T: Default> Default for Lazy<T> {
    /// Creates a new lazy value using `Default` as the initializing function.
    fn default() -> Lazy<T> {
        Lazy::new(T::default)
    }
}

struct Canary<'a>(&'a AtomicBool);

impl<'a> Drop for Canary<'a> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release)
    }
}
