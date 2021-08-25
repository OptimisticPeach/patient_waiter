use std::sync::atomic::{AtomicUsize, Ordering, AtomicBool};
use parking_lot_core::{park, unpark_all, DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN};

pub struct PatientWaiter(AtomicBool);

impl PatientWaiter {
    pub fn new() -> Self { PatientWaiter(AtomicBool::new(false)) }

    pub fn notify(&self) {
        if self.0.load(Ordering::Relaxed) {
            self.notify_slow();
        }
    }

    #[cold]
    fn notify_slow(&self) {
        let key = self as usize;
        unsafe {
            unpark_all(
                key,
                DEFAULT_UNPARK_TOKEN,
            );
        }

        self.0.store(false, Ordering::Relaxed);
    }

    pub fn wait(&self, hook: impl FnOnce()) {
        let key = self as usize;

        let validate = || true;
        let timeout = |_, _| {};

        let waiting = &self.0;

        let hook = move || {
            waiting.store(true, Ordering::Relaxed);
            hook();
        };

        unsafe {
            park(
                key,
                validate,
                hook,
                timeout,
                DEFAULT_PARK_TOKEN,
                None,
            );
        }
    }

    pub fn wait_until(&self, hook: impl FnOnce(), mut is_done: impl FnMut() -> bool, mut lock_and_validate: impl FnMut() -> bool) {
        let key = self as usize;

        let validate = || true;
        let timeout = |_, _| {};

        let waiting = &self.0;

        let hook = move || {
            waiting.store(true, Ordering::Relaxed);
            hook();
        };

        unsafe {
            park(
                key,
                validate,
                hook,
                timeout,
                DEFAULT_PARK_TOKEN,
                None,
            );
        }

        let hook = || {
            waiting.store(true, Ordering::Relaxed);
        };

        loop {
            if !is_done() {
                unsafe {
                    park(
                        key,
                        validate,
                        hook,
                        timeout,
                        DEFAULT_PARK_TOKEN,
                        None,
                    );
                }
            } else if lock_and_validate() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
