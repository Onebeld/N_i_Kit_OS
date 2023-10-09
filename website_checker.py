import socket
import ssl
from urllib.parse import urlparse

import requests


def get_website_code_response(website: str):
    if website.count("https://") == 0 and website.count("http://") == 0:
        website = "https://" + website

    request = requests.get(website, verify=False)

    return request.status_code.numerator


def get_ssl_certificate(website: str):
    if website.count("https://") == 0 and website.count("http://") == 0:
        website = "https://" + website

    parsed_uri = urlparse(website)

    try:
        context = ssl.create_default_context()
        sock = context.wrap_socket(socket.socket(), server_hostname=parsed_uri.netloc)
        sock.connect((parsed_uri.netloc, 443))
        certificate = sock.getpeercert()

        issuer = dict(x[0] for x in certificate['issuer'])

        return issuer

    except:
        return None
