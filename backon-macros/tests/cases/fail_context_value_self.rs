use backon_macros::backon;

struct Owner;

impl Owner {
    #[backon(context = true)]
    fn take(self, payload: String) -> Result<String, ()> {
        Ok(payload)
    }
}

fn main() {}
