use backon_macros::backon;

#[backon(backoff = backon::ExponentialBuilder::default)]
fn attempt_sync() -> Result<i32, &'static str> {
    Ok(7)
}

fn main() {
    let _ = attempt_sync();
}
