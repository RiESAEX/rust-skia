use std::io::Read;
use std::{env, io};

use ureq::Proxy;

/// Download a file from the given URL and return the data.
pub fn download(url: impl AsRef<str>) -> io::Result<Vec<u8>> {
    let resp = if let Ok(proxy) = env::var("https_proxy").or_else(|_| env::var("HTTPS_PROXY")) {
        println!("{}",&proxy);
        if let Ok(proxy) = Proxy::new(proxy) {
            let agent = ureq::AgentBuilder::new().proxy(proxy).build();
            println!("proxy");
            agent.get(url.as_ref()).call()
        } else {
            ureq::get(url.as_ref()).call()
        }
    } else {
        ureq::get(url.as_ref()).call()
    };

    match resp {
        Ok(resp) => {
            let mut reader = resp.into_reader();
            let mut data = Vec::new();
            reader.read_to_end(&mut data)?;
            Ok(data)
        }
        Err(error) => Err(io::Error::new(io::ErrorKind::Other, error.to_string())),
    }
}
