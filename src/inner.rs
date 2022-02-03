use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Waker;

#[cfg(feature = "futures-util")]
use futures_util::task::AtomicWaker;

#[cfg(all(not(feature = "futures-util"), feature = "atomic-waker"))]
use atomic_waker::AtomicWaker;

#[cfg(all(not(feature = "atomic-waker"), not(feature = "futures-util")))]
compile_error!("Please select an AtomicWaker implementation: futures-util or atomic-waker");

#[repr(C)]
struct Inner {
    refcount: AtomicUsize,
    waker: AtomicWaker,
}

pub struct InnerPtr(NonNull<Inner>);

unsafe impl Send for InnerPtr {}
unsafe impl Sync for InnerPtr {}

impl InnerPtr {
    pub fn new() -> Self {
        let p = Box::into_raw(Box::new(Inner {
            refcount: AtomicUsize::new(1),
            waker: AtomicWaker::new(),
        }));
        unsafe { Self(NonNull::new_unchecked(p)) }
    }

    #[inline(always)]
    fn deref(&self) -> &Inner {
        unsafe { self.0.as_ref() }
    }

    pub fn count(&self) -> usize {
        self.deref().refcount.load(Ordering::Relaxed) - 1
    }

    pub fn register_waker(&self, waker: &Waker) {
        self.deref().waker.register(waker);
    }
}

impl Clone for InnerPtr {
    fn clone(&self) -> Self {
        let old_refcount = self.deref().refcount.fetch_add(1, Ordering::Relaxed);
        #[cfg(not(target_pointer_width = "64"))]
        {
            const MAX_REFCOUNT: usize = (isize::MAX) as usize;
            if old_refcount > MAX_REFCOUNT {
                std::process::abort();
            }
        }
        #[cfg(target_pointer_width = "64")]
        {
            let _ = old_refcount; // the overflow takes hundreds of years
        }
        Self(self.0)
    }
}

impl Drop for InnerPtr {
    fn drop(&mut self) {
        #[inline(never)]
        unsafe fn drop_slow(this: *mut Inner, old_refcount: usize) {
            match old_refcount {
                2 => (*this).waker.wake(),
                1 => drop(Box::from_raw(this)),
                _ => {}
            }
        }

        let old_refcount = self.deref().refcount.fetch_sub(1, Ordering::Release);
        if old_refcount > 2 {
            return;
        }
        self.deref().refcount.load(Ordering::Acquire);
        unsafe { drop_slow(self.0.as_ptr(), old_refcount) }
    }
}
