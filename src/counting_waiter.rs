use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use parking_lot_core::{DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN, park, unpark_all, unpark_one, UnparkToken};
use crate::ValidateResult;

const UNPARK_ONE: UnparkToken = UnparkToken(3);
const UNPARK_ALL: UnparkToken = UnparkToken(4);

pub struct CountingWaiter(AtomicU64, AtomicBool);

pub enum CountingToken {
    ByAll(u64),
    ByOne(u64),
}

impl CountingToken {
    fn inner(&self) -> u64 {
        match self {
            CountingToken::ByAll(x) | CountingToken::ByOne(x) => *x
        }
    }
}

impl CountingWaiter {
    pub fn new() -> Self {
        CountingWaiter(AtomicU64::new(1), AtomicBool::new(false))
    }

    pub fn notify(&self) {
        if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
            panic!("Counter overflowed. Don't run me for a year!")
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
        CountingToken::ByAll(self.0.load(Ordering::Acquire))
    }

    pub fn wait_token(&self, token: &mut CountingToken) {
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

    /// Hook cannot wait on anything else, or panic.
    pub unsafe fn wait_hooked(&self, hook: impl FnOnce()) {
        let key = self as *const _ as usize;

        let validate = || true;
        let timeout = |_, _| {};

        let waiting = &self.1;

        let hook = move || {
            waiting.store(true, Ordering::Relaxed);
            hook();
        };

        // SAFETY: Caller promised that hook will not wait on anything or panic.
        park(key, validate, hook, timeout, DEFAULT_PARK_TOKEN, None);
    }

    /// Hook cannot wait on anything else, or panic.
    pub unsafe fn wait_hooked_until(
        &self,
        mut unlock: impl FnMut(),
        mut is_done: impl FnMut() -> bool,
        mut lock_and_validate: impl FnMut() -> ValidateResult,
    ) {
        let key = self as *const _ as usize;

        let validate = || true;
        let timeout = |_, _| {};

        let waiting = &self.1;

        let hook = || {
            waiting.store(true, Ordering::Relaxed);
            unlock();
        };

        // SAFETY: Caller promised that hook will not wait on anything or panic.
        park(key, validate, hook, timeout, DEFAULT_PARK_TOKEN, None);

        let before_sleep = || {
            waiting.store(true, Ordering::Relaxed);
        };

        loop {
            if !is_done() {
                // SAFETY: Caller promised that hook will not wait on anything or panic.
                park(key, validate, before_sleep, timeout, DEFAULT_PARK_TOKEN, None);
            } else {
                match lock_and_validate() {
                    ValidateResult::Abort { run_hook: true } => {
                        unlock();
                        return;
                    }
                    ValidateResult::Abort { run_hook: false } | ValidateResult::Success => return,
                    ValidateResult::Retry => unlock(),
                }
            }
        }
    }
}
