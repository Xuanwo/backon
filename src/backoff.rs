use std::time::Duration;

pub trait Backoff: Iterator<Item = Duration> + Clone {}
impl<T> Backoff for T where T: Iterator<Item = Duration> + Clone {}
