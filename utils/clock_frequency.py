#!/usr/bin/env python3
from bs4 import BeautifulSoup
import json


FREQUENCY_FILTER_ID = 2150

# min/max are taken from SAM L21 datasheet
MIN_FREQUENCY = 400_000
MAX_FREQUENCY = 32_000_000
REQUESTED_DIVISOR = 77_500

EPSILON = 1e-30


def main():
    with open("172.html", "rb") as f:
        html_str = f.read()
    html = BeautifulSoup(html_str)
    data_script = html.find("script", id="__NEXT_DATA__")
    data = data_script.text

    page = json.loads(data)

    frequency_filter = next(
        flt
        for flt in page["props"]["pageProps"]["envelope"]["data"]["filters"]
        if flt["key"] == str(FREQUENCY_FILTER_ID)
    )
    for option in frequency_filter["options"]:
        (raw_value, unit) = option["value"].split(" ", 1)
        unit = unit.removesuffix("Hz")

        si_factor = {
            "n": 1e-9,
            "µ": 1e-6,
            "m": 1e-3,
            "": 1,
            "k": 1e3,
            "M": 1e6,
            "G": 1e9,
        }[unit]

        value = float(raw_value) * si_factor

        if value < MIN_FREQUENCY:
            continue
        if value > MAX_FREQUENCY:
            continue

        if abs(value % REQUESTED_DIVISOR) < EPSILON:
            print(f"{option['value']} ({option['productCount']})")


if __name__ == "__main__":
    main()
