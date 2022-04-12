use std::time::Duration;

pub trait Policy: Iterator<Item = Duration> {}
impl<T> Policy for T where T: Iterator<Item = Duration> {}
