use std::io::{Error};
use checkssl::{Cert, CheckSSL};
use http::{StatusCode, Uri};

pub struct Website {
    url: String
}

impl Website {
    pub fn new(url: &str) -> Website {
        Website {
            url: url.to_string()
        }
    }

    pub fn get_ssl_certificate(&self) -> Result<Cert, Error> {
        let uri = self.url.parse::<Uri>().unwrap();
        CheckSSL::from_domain(uri.host().unwrap())
    }

    pub async fn get_request_code(&self) -> Option<StatusCode> {
        let uri = self.url.parse::<Uri>().unwrap();
        let res = reqwest::get(uri.to_string()).await;

        return match res {
            Ok(response) => {
                Some(response.status())
            }
            Err(_) => {
                None
            }
        }
    }
}

#[cfg(test)]
mod website_checker_tests {
    use crate::website_checker::Website;

    static GOOGLE: &str = "https://google.com";
    static YOUTUBE: &str = "youtube.com";
    static PYTHON: &str = "www.python.org";
    static RSVPU: &str = "https://rsvpu.ru/programs/bakalavriat";
    static HTTP: &str = "http://info.cern.ch/";

    #[test]
    fn test_get_certificate() {
        let google_certificate = Website::new(GOOGLE).get_ssl_certificate();
        println!("Organization name from google.com: {}", google_certificate.unwrap().intermediate.organization);

        let youtube_certificate = Website::new(YOUTUBE).get_ssl_certificate();
        println!("Organization name from youtube.com: {}", youtube_certificate.unwrap().intermediate.organization);

        let python_certificate = Website::new(PYTHON).get_ssl_certificate();
        println!("Organization name from python.org: {}", python_certificate.unwrap().intermediate.organization);

        let http_certificate = Website::new(HTTP).get_ssl_certificate();
        println!("Organization name from info.cern.ch: {}", http_certificate.unwrap().intermediate.organization);

        let rsvpu_certificate = Website::new(RSVPU).get_ssl_certificate();

        match rsvpu_certificate {
            Ok(cert) => {
                println!("Organization name from rsvpu.ru: {}", cert.intermediate.organization);
            }
            Err(_) => {
                println!("Organization name from rsvpu.ru: no data");
            }
        }

        assert!(true)
    }
}