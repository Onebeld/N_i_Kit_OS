use checkssl::{Cert, CheckSSL};
use http::{Uri};
use lazy_static::lazy_static;
use regex::Regex;

const HTTP_S_REGEX: &str = "^(http|https)://";

pub struct SiteInformation {
    pub status_code: u16,
    pub has_robots: u16,
    pub has_sitemap: u16,
    pub duration: u128,
    pub certificate: Option<Cert>
}

lazy_static! {
    static ref RE_HTTP_S: Regex = {
        Regex::new(HTTP_S_REGEX).unwrap()
    };
}

pub async fn get_request_code(url: &str) -> Result<u16, reqwest::Error> {
    let resp = reqwest::get(url).await?;
    let status_code = resp.status().as_u16();

    Ok(status_code)
}

pub async fn get_site_information(url: &str) -> Result<SiteInformation, reqwest::Error> {
    let client = reqwest::Client::new();

    let time_now = std::time::Instant::now();
    let resp_site = reqwest::get(url).await?;
    let elapsed_time = time_now.elapsed();

    let uri = url.parse::<Uri>().unwrap();
    let cert = CheckSSL::from_domain(uri.host().unwrap());

    let resp_robots = client.get(format!("{}://{}/robots.txt", uri.scheme_str().unwrap(), uri.host().unwrap())).send().await?;
    let resp_sitemap = client.get(format!("{}://{}/sitemap.xml", uri.scheme_str().unwrap(), uri.host().unwrap())).send().await?;

    Ok(SiteInformation {
        status_code: resp_site.status().as_u16(),
        duration: elapsed_time.as_millis(),
        certificate: cert.ok(),
        has_robots: resp_robots.status().as_u16(),
        has_sitemap: resp_sitemap.status().as_u16()
    })
}

pub fn has_http_s(url: &str) -> bool {
    RE_HTTP_S.is_match(url)
}


#[cfg(test)]
mod website_checker_tests {
    use crate::website;

    static GOOGLE: &str = "https://google.com";
    static YOUTUBE: &str = "youtube.com";
    static PYTHON: &str = "www.python.org";
    static RSVPU: &str = "https://rsvpu.ru/programs/bakalavriat";
    static HTTP: &str = "http://info.cern.ch/";

    #[tokio::test]
    async fn test_get_certificate() {
        let google_information = website::get_site_information(GOOGLE).await;

        match google_information {
            Ok(info) => {
                println!("Status code: {}", info.status_code);

                match info.certificate {
                    Some(cert) => {
                        println!("Organization name: {}", cert.intermediate.organization)
                    }
                    None => {
                        println!("Organization name: none")
                    }
                }

                println!("Duration: {}", info.duration);
                println!("Has robots.txt: {}", info.has_robots);
                println!("Has sitemap.xml: {}", info.has_sitemap);

                assert!(true)
            }
            Err(e) => {
                println!("Failed to verify the site: {}", e);

                assert!(false)
            }
        }
    }

    #[tokio::test]
    async fn test_get_request_code() {
        let google_request = website::get_request_code("https://latitude.google.com/").await.unwrap();
        println!("Request code: {}", google_request);

        assert!(true)
    }
}