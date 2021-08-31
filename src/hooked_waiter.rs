use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot_core::{DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN, park, unpark_all, unpark_one};

use crate::ValidateResult;

pub struct HookedWaiter(AtomicBool);

impl HookedWaiter {
    pub fn new() -> Self {
        HookedWaiter(AtomicBool::new(false))
    }

    pub fn notify(&self) {
        if self.0.load(Ordering::Relaxed) {
            self.notify_slow();
        }
    }

    #[cold]
    fn notify_slow(&self) {
        let key = self as *const _ as usize;
        unsafe {
            unpark_all(key, DEFAULT_UNPARK_TOKEN);
        }

        self.0.store(false, Ordering::Relaxed);
    }

    pub fn notify_one(&self) {
        if self.0.load(Ordering::Relaxed) {
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
                        self.0.store(false, Ordering::Relaxed);
                    }
                    DEFAULT_UNPARK_TOKEN
                }
            )
        };
    }

    /// Hook cannot wait on anything else, or panic.
    pub unsafe fn wait(&self, hook: impl FnOnce()) {
        let key = self as *const _ as usize;

        let validate = || true;
        let timeout = |_, _| {};

        let waiting = &self.0;

        let hook = move || {
            waiting.store(true, Ordering::Relaxed);
            hook();
        };

        // SAFETY: Caller promised that hook will not wait on anything or panic.
        park(key, validate, hook, timeout, DEFAULT_PARK_TOKEN, None);
    }

    /// Hook cannot wait on anything else, or panic.
    pub unsafe fn wait_until(
        &self,
        mut unlock: impl FnMut(),
        mut is_done: impl FnMut() -> bool,
        mut lock_and_validate: impl FnMut() -> ValidateResult,
    ) {
        let key = self as *const _ as usize;

        let validate = || true;
        let timeout = |_, _| {};

        let waiting = &self.0;

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
