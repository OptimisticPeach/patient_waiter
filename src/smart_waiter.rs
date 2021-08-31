// use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
//
// use parking_lot_core::{DEFAULT_PARK_TOKEN, park};
//
// pub struct SmartWaiter {
//     count: AtomicU64,
//     any: AtomicBool,
// }
//
// pub struct SmartToken(u64);
//
// impl SmartWaiter {
//     pub fn new() -> Self {
//         SmartWaiter {
//             count: AtomicU64::new(1),
//             any: AtomicBool::new(false),
//         }
//     }
//
//     pub fn token(&self) -> SmartToken {
//         SmartToken(self.count.load(Ordering::Acquire))
//     }
//
//     /// Hook may not panic nor call into parking_lot.
//     pub unsafe fn wait(
//         &self,
//         SmartToken(token): &mut SmartToken,
//         mut unlock: impl FnMut(),
//         mut is_done: impl FnMut(),
//         mut lock_and_validate: impl FnMut(),
//     ) {
//         // let key = self as *const _ as usize;
//         //
//         // let validate = || {
//         //     if self.0.load(Ordering::Relaxed) > *token {
//         //         return false;
//         //     }
//         //     self.1.store(true, Ordering::Relaxed);
//         //     hook();
//         //     true
//         // };
//         // let timeout = |_, _| {};
//         //
//         // let hook = || {};
//         //
//         // // SAFETY: hook will never panic, nor park the thread.
//         // unsafe {
//         //     park(key, validate, hook, timeout, DEFAULT_PARK_TOKEN, None);
//         // }
//         //
//         // *token = self.0.load(Ordering::Relaxed)
//     }
// }
