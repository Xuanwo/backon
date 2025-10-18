use backon_macros::backon;

struct Client;

impl Client {
    #[backon(context = true)]
    fn fetch(&self) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
