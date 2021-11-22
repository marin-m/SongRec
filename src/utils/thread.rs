use std::thread;

pub fn spawn_big_thread<F, T>(argument: F) -> ()
    where
        F: std::ops::FnOnce() -> T,
        F: std::marker::Send + 'static,
        T: std::marker::Send + 'static {
    thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(argument).unwrap();
}
