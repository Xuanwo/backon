use backon_macros::backon;

struct Counter {
    value: usize,
}

impl Counter {
    #[backon(context = true)]
    async fn bump(&mut self, payload: String) -> Result<(), ()> {
        let _ = payload;
        Err(())
    }
}

fn main() {}
