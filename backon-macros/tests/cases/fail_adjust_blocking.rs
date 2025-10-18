use backon_macros::backon;

fn adjuster(_: &str, _: Option<core::time::Duration>) -> Option<core::time::Duration> {
    None
}

#[backon(adjust = adjuster)]
fn invalid_blocking() -> Result<(), &'static str> {
    Ok(())
}

fn main() {
    let _ = invalid_blocking();
}
