use backon_macros::backon;

#[backon(context = true)]
async fn invalid_pattern((a, b): (i32, i32)) -> Result<(), ()> {
    let _ = (a, b);
    Ok(())
}

fn main() {}
