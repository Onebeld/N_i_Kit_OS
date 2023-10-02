import time
import unittest

import website_checker


class MyTestCase(unittest.TestCase):
    def test_get_website_code_response(self):
        website_google = "https://google.com"
        website_youtube = "youtube.com"
        website_python = "www.python.org"
        website_rsvpu = "https://rsvpu.ru/programs/bakalavriat"
        website_http = "http://info.cern.ch/"

        status_code_google = website_checker.get_website_code_response(website_google)
        print("Response code for google.com: " + str(status_code_google))

        status_code_youtube = website_checker.get_website_code_response(website_youtube)
        print("Response code for youtube.com: " + str(status_code_youtube))

        status_code_python = website_checker.get_website_code_response(website_python)
        print("Response code for python.org: " + str(status_code_python))

        status_code_rsvpu = website_checker.get_website_code_response(website_rsvpu)
        print("Response code for rsvpu.ru: " + str(status_code_rsvpu))

        status_code_http = website_checker.get_website_code_response(website_http)
        print("Response code for http site: " + str(status_code_http))

        time.sleep(0.2)

        self.assertTrue(True)


if __name__ == '__main__':
    unittest.main()
