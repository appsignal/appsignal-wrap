use reqwest::{Client, ClientBuilder};

use ::log::{debug, trace};

use crate::package::{NAME, VERSION};

pub fn client() -> Client {
    ClientBuilder::new()
        .user_agent(format!("{NAME}/{VERSION}"))
        .build()
        .unwrap()
}

pub async fn send_request(request: Result<reqwest::Request, reqwest::Error>) {
    let request = match request {
        Ok(request) => request,
        Err(err) => {
            debug!("error creating request: {}", err);
            return;
        }
    };

    match client().execute(request.try_clone().unwrap()).await {
        Ok(response) => {
            if !response.status().is_success() {
                debug!("request failed with status: {}", response.status());
            } else {
                trace!("request successful: {}", request.url());
            }
        }
        Err(err) => {
            debug!("error sending request: {:?}", err);
        }
    };
}
