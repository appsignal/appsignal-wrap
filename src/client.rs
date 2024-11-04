use reqwest::{Client, ClientBuilder};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn client() -> Client {
    ClientBuilder::new()
        .user_agent(format!("{NAME}/{VERSION}"))
        .build()
        .unwrap()
}
