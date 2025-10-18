use backon_macros::backon;

struct Client {
    value: i32,
}

impl Client {
    #[backon]
    fn fetch_sync(&self) -> Result<i32, &'static str> {
        Ok(self.value)
    }

    #[backon]
    async fn fetch_async(&self) -> Result<i32, &'static str> {
        let value = self.fetch_sync()?;
        Ok(value)
    }
}

fn main() {
    let client = Client { value: 7 };
    let _ = client.fetch_sync();
    let _ = client.fetch_async();
}
