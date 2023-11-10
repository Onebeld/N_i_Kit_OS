use std::io::{Error};
use checkssl::{Cert, CheckSSL};
use http::{Uri};
use lazy_static::lazy_static;
use regex::Regex;

const HTTP_S_REGEX: &str = "^(http|https)://";

lazy_static! {
    static ref RE_HTTP_S: Regex = {
        Regex::new(HTTP_S_REGEX).unwrap()
    };
}

pub struct Website { }

impl Website {
    pub fn get_ssl_certificate(url: &str) -> Result<Cert, Error> {
        let uri = url.parse::<Uri>().unwrap();
        CheckSSL::from_domain(uri.host().unwrap())
    }

    pub fn get_request_code(url: &str) -> Result<u16, reqwest::Error> {
        let client = reqwest::blocking::Client::new();

        let resp = client.get(url).send()?;

        let status_code = resp.status().as_u16();

        Ok(status_code)
    }

    pub fn has_http_s(url: &str) -> bool {
        RE_HTTP_S.is_match(url)
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
        let google_certificate = Website::get_ssl_certificate(GOOGLE);
        println!("Organization name from google.com: {}", google_certificate.unwrap().intermediate.organization);

        let youtube_certificate = Website::get_ssl_certificate(YOUTUBE);
        println!("Organization name from youtube.com: {}", youtube_certificate.unwrap().intermediate.organization);

        let python_certificate = Website::get_ssl_certificate(PYTHON);
        println!("Organization name from python.org: {}", python_certificate.unwrap().intermediate.organization);

        let http_certificate = Website::get_ssl_certificate(HTTP);
        println!("Organization name from info.cern.ch: {}", http_certificate.unwrap().intermediate.organization);

        let rsvpu_certificate = Website::get_ssl_certificate(RSVPU);

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

    #[test]
    fn test_get_request_code() {
        let google_request = Website::get_request_code("https://latitude.google.com/").unwrap();
        println!("Request code: {}", google_request);

        assert!(true)
    }
}