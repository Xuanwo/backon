use std::time::Duration;

pub trait Policy: Iterator<Item = Duration> + Clone {}
impl<T> Policy for T where T: Iterator<Item = Duration> + Clone {}
