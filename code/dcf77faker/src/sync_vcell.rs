//! Implementation of a volatile cell that pretends to implement [`Sync`].


use vcell::VolatileCell;


/// A volatile cell that pretends to implement [`Sync`].
#[repr(transparent)]
pub(crate) struct SyncVolatileCell<T> {
    cell: VolatileCell<T>,
}
impl<T> SyncVolatileCell<T> {
    pub const fn new(value: T) -> Self {
        Self { cell: VolatileCell::new(value) }
    }

    #[inline(always)]
    pub fn get(&self) -> T where T: Copy {
        self.cell.get()
    }

    /// Sets the contained value
    #[inline(always)]
    pub fn set(&self, value: T) where T: Copy {
        self.cell.set(value)
    }

    /// Returns a raw pointer to the underlying data in the cell
    #[inline(always)]
    pub fn as_ptr(&self) -> *mut T {
        self.cell.as_ptr()
    }
}
unsafe impl<T> Send for SyncVolatileCell<T> {
}
unsafe impl<T> Sync for SyncVolatileCell<T> {
}
