use backon_macros::backon;

#[backon(context = true)]
async fn attempt_with_context(value: String) -> Result<String, &'static str> {
    if value.is_empty() {
        Err("empty")
    } else {
        let output = value.clone();
        Ok(output)
    }
}

fn main() {
    let _ = attempt_with_context("data".to_string());
}
