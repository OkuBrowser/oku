// Note: We take `&*mut T` instead of just `*mut T` to tie the lifetime of all the returned items
// to the lifetime of the pointer for some extra safety.
pub(crate) unsafe fn linked_list_iter_fn<T>(
    ptr: &*mut T,
    next: fn(&T) -> *mut T,
) -> impl Iterator<Item = &T> {
    let mut ptr = ptr.cast_const();

    std::iter::from_fn(move || {
        let cur = unsafe { ptr.as_ref()? };
        ptr = next(cur);
        Some(cur)
    })
}

// The `Next` element is always the same, so use a macro to avoid the repetition.
macro_rules! linked_list_iter {
    ($ptr:expr) => {
        $crate::os::windows::macros::linked_list_iter_fn($ptr, |cur| cur.Next)
    };
}

pub(crate) use linked_list_iter;
