import requests


def get_website_code_response(website: str):
    if website.count("https://") == 0 and website.count("http://") == 0:
        website = "https://" + website

    request = requests.get(website, verify=False)

    return request.status_code.numerator
