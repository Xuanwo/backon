use backon_macros::backon;

#[backon]
async fn attempt_async() -> Result<i32, &'static str> {
    Ok(42)
}

fn main() {
    let _ = attempt_async();
}
