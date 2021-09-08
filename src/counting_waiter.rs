use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use parking_lot_core::{DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN, park, unpark_all, unpark_one, UnparkToken};
use crate::ValidateResult;

const UNPARK_ONE: UnparkToken = UnparkToken(3);
const UNPARK_ALL: UnparkToken = UnparkToken(4);

pub struct CountingWaiter(AtomicU64, AtomicBool);

pub struct CountingToken(u64);

impl CountingWaiter {
    pub fn new() -> Self {
        CountingWaiter(AtomicU64::new(1), AtomicBool::new(false))
    }

    pub fn notify(&self) {
        if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
            panic!("Counter overflowed.")
        }
        if self.1.load(Ordering::Relaxed) {
            self.notify_slow();
        }
    }

    #[cold]
    fn notify_slow(&self) {
        let key = self as *const _ as usize;
        unsafe {
            unpark_all(key, DEFAULT_UNPARK_TOKEN);
        }

        self.1.store(false, Ordering::Relaxed);
    }

    pub fn notify_one(&self) {
        if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
            panic!("Counter overflowed. Don't run me for a year!")
        }
        if self.1.load(Ordering::Relaxed) {
            self.notify_one_slow();
        }
    }

    #[cold]
    fn notify_one_slow(&self) {
        let key = self as *const _ as usize;
        unsafe {
            unpark_one(
                key,
                |unpark_result| {
                    if !unpark_result.have_more_threads {
                        self.1.store(false, Ordering::Relaxed);
                    }
                    DEFAULT_UNPARK_TOKEN
                }
            )
        };
    }

    pub fn token(&self) -> CountingToken {
        CountingToken(self.0.load(Ordering::Acquire))
    }

    pub fn wait_token(&self, CountingToken(token): &mut CountingToken) {
        let key = self as *const _ as usize;

        let validate = || {
            if self.0.load(Ordering::Relaxed) > *token {
                return false;
            }
            self.1.store(true, Ordering::Relaxed);
            true
        };
        let timeout = |_, _| {};

        let hook = || {};

        // SAFETY: hook will never panic, nor park the thread.
        unsafe {
            park(key, validate, hook, timeout, DEFAULT_PARK_TOKEN, None);
        }

        *token = self.0.load(Ordering::Relaxed)
    }
}
