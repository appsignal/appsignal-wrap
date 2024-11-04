use reqwest::{Client, ClientBuilder};

use crate::package::{NAME, VERSION};

pub fn client() -> Client {
    ClientBuilder::new()
        .user_agent(format!("{NAME}/{VERSION}"))
        .build()
        .unwrap()
}
