import unittest
from datetime import datetime, timedelta, timezone

from sunsetRollercoaster.crawler.fuel_price import NationwideFuelPriceCrawler


class NationwideFuelPriceCrawlerTest(unittest.TestCase):
    def test_parse_weekly_prices(self):
        payload = {
            "res": "01",
            "msg": "",
            "data": {
                "gasoline": [
                    {
                        "Oil92": 30.48,
                        "Oil95": 31.97,
                        "Oil98": 33.97,
                        "Oilchai": 29.23,
                        "SurDate": "2026/07/26 ~ 2026/08/01",
                    }
                ]
            },
        }

        result = NationwideFuelPriceCrawler._parse(payload)

        self.assertEqual(len(result), 1)
        taiwan_tz = timezone(timedelta(hours=8))
        self.assertEqual(
            result[0].period_start,
            datetime(2026, 7, 26, tzinfo=taiwan_tz),
        )
        self.assertEqual(
            result[0].period_end,
            datetime(2026, 8, 1, tzinfo=taiwan_tz),
        )
        self.assertEqual(result[0].period_start.utcoffset(), timedelta(hours=8))
        self.assertEqual(result[0].unleaded_92, 30.48)
        self.assertEqual(result[0].unleaded_95, 31.97)
        self.assertEqual(result[0].unleaded_98, 33.97)
        self.assertEqual(result[0].super_diesel, 29.23)

    def test_parse_rejects_unsuccessful_response(self):
        with self.assertRaisesRegex(ValueError, "request failed"):
            NationwideFuelPriceCrawler._parse(
                {"res": "", "msg": "upstream error", "data": {}}
            )

    def test_parse_rejects_invalid_period(self):
        payload = {
            "res": "01",
            "msg": "",
            "data": {
                "gasoline": [
                    {
                        "Oil92": 30.48,
                        "Oil95": 31.97,
                        "Oil98": 33.97,
                        "Oilchai": 29.23,
                        "SurDate": "2026/08/01",
                    }
                ]
            },
        }

        with self.assertRaisesRegex(ValueError, "invalid fuel price period"):
            NationwideFuelPriceCrawler._parse(payload)

    def test_parse_rejects_invalid_price(self):
        payload = {
            "res": "01",
            "msg": "",
            "data": {
                "gasoline": [
                    {
                        "Oil92": None,
                        "Oil95": 31.97,
                        "Oil98": 33.97,
                        "Oilchai": 29.23,
                        "SurDate": "2026/07/26 ~ 2026/08/01",
                    }
                ]
            },
        }

        with self.assertRaisesRegex(ValueError, "Oil92"):
            NationwideFuelPriceCrawler._parse(payload)


if __name__ == "__main__":
    unittest.main()
