pub mod hooked_waiter;
pub mod counting_waiter;
pub mod smart_waiter;

pub enum ValidateResult {
    Success,
    Abort { run_hook: bool },
    Retry,
}
