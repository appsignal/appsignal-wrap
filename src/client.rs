use crate::version::VERSION;
use reqwest::{Client, ClientBuilder};

pub fn client() -> Client {
  ClientBuilder::new()
      .user_agent(format!("appsignal-wrap/{}", VERSION))
      .build()
      .unwrap()
}
