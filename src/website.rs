use checkssl::{Cert, CheckSSL};
use http::{Uri};
use lazy_static::lazy_static;
use regex::Regex;

const HTTP_OR_HTTPS_REGEX: &str = "^(http|https)://";

/// Represents information about a website.
pub struct SiteInformation {
    pub status_code: u16,
    pub has_robots: u16,
    pub has_sitemap: u16,
    pub duration: u128,
    pub certificate: Option<Cert>
}

lazy_static! {
    static ref RE_HTTP_OR_HTTPS: Regex = {
        Regex::new(HTTP_OR_HTTPS_REGEX).unwrap()
    };
}

/// Sends a GET request to the specified URL and returns the status code.
///
/// # Arguments
///
/// * `url` - A string slice that holds the URL of the API endpoint.
///
/// # Returns
///
/// * An `Ok` variant containing the status code as a `u16` if the request is successful.
/// * An `Err` variant containing a `reqwest::Error` if an error occurs during the request.
///
/// # Examples
///
/// ```
/// use reqwest::Error;
///
/// async fn example() -> Result<u16, Error> {
///     let url = "https://example.com";
///     let status_code = get_request_code(url).await?;
///     println!("Status code: {}", status_code);
///     Ok(status_code)
/// }
/// ```
pub async fn get_request_code(url: &str) -> Result<u16, reqwest::Error> {
    let resp = reqwest::get(url).await?;
    let status_code = resp.status().as_u16();

    Ok(status_code)
}

/// Fetches site information for a given URL.
///
/// The function makes use of the `reqwest` crate to perform HTTP requests.
/// It retrieves the following information about the site:
/// - Status code of the main page response
/// - Duration of the request in milliseconds
/// - Existence of SSL certificate for the domain
/// - Status code of the response for the robots.txt file
/// - Status code of the response for the sitemap.xml file
///
/// # Arguments
///
/// * `url` - A string slice representing the URL of the site to fetch information for.
///
/// # Returns
///
/// A `Result` containing a `SiteInformation` struct on success, or a `reqwest::Error` on failure.
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

/// Checks if a given URL has either "http" or "https" protocol.
///
/// # Arguments
///
/// * `url` - A string slice representing the URL to be checked.
///
/// # Returns
///
/// * `bool` - "true" if the URL has either "http" or "https" protocol, "false" otherwise.
///
/// # Example
///
/// ```
/// assert_eq!(has_http_or_https("http://example.com"), true);
/// assert_eq!(has_http_or_https("https://example.com"), true);
/// assert_eq!(has_http_or_https("ftp://example.com"), false);
/// ```
pub fn has_http_or_https(url: &str) -> bool {
    RE_HTTP_OR_HTTPS.is_match(url)
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