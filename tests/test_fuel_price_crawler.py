import unittest
from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, Mock

import httpx

from sunsetRollercoaster.crawler.fuel_price import NationwideFuelPriceCrawler


class NationwideFuelPriceCrawlerTest(unittest.TestCase):
    def test_parse_weekly_prices(self):
        fuel_payload = {
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
        crude_payload = {
            "res": "",
            "msg": "",
            "data": {
                "crudeoil": [
                    {
                        "WestT": 84.51,
                        "Dubit": None,
                        "Burant": 91.79,
                        "SurDate": "2026/07/26 ~ 2026/08/01",
                    }
                ]
            },
        }

        result = NationwideFuelPriceCrawler._parse_combined(
            fuel_payload,
            crude_payload,
        )

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
        self.assertEqual(result[0].west_texas, 84.51)
        self.assertIsNone(result[0].dubai)
        self.assertEqual(result[0].brent, 91.79)

    def test_parse_accepts_historical_negative_west_texas_price(self):
        fuel_payload = {
            "res": "01",
            "msg": "",
            "data": {
                "gasoline": [
                    {
                        "Oil92": 20.0,
                        "Oil95": 21.0,
                        "Oil98": 23.0,
                        "Oilchai": 18.0,
                        "SurDate": "2020/04/19 ~ 2020/04/25",
                    }
                ]
            },
        }
        crude_payload = {
            "res": "01",
            "msg": "",
            "data": {
                "crudeoil": [
                    {
                        "WestT": -8.86,
                        "Dubit": 17.17,
                        "Burant": 21.2,
                        "SurDate": "2020/04/19 ~ 2020/04/25",
                    }
                ]
            },
        }

        result = NationwideFuelPriceCrawler._parse_combined(
            fuel_payload,
            crude_payload,
        )

        self.assertEqual(result[0].west_texas, -8.86)

    def test_parse_keeps_domestic_prices_without_crude_oil_period(self):
        fuel_payload = {
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
        crude_payload = {
            "res": "01",
            "msg": "",
            "data": {"crudeoil": []},
        }

        result = NationwideFuelPriceCrawler._parse_combined(
            fuel_payload,
            crude_payload,
        )

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0].unleaded_95, 31.97)
        self.assertIsNone(result[0].west_texas)
        self.assertIsNone(result[0].dubai)
        self.assertIsNone(result[0].brent)

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


def weekly_payloads():
    return (
        {
            "res": "01",
            "data": {
                "endweek": "1395",
                "gasoline": [
                    {
                        "Oil92": 31.18,
                        "Oil95": 32.67,
                        "Oil98": 34.68,
                        "Oilchai": 29.84,
                        "SurDate": "2026/09/20 ~ 2026/09/26",
                    },
                    {
                        "Oil92": 31.17,
                        "Oil95": 32.66,
                        "Oil98": 34.67,
                        "Oilchai": 29.83,
                        "SurDate": "2026/09/13 ~ 2026/09/19",
                    },
                ],
            },
        },
        {
            "res": "",
            "data": {
                "crudeoil": [
                    {
                        "WestT": 103.54,
                        "Dubit": 120.47,
                        "Burant": 122.56,
                        "SurDate": "2026/09/13 ~ 2026/09/19",
                    }
                ]
            },
        },
    )


class FuelPriceSyncTest(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.fuel, self.crude = weekly_payloads()
        self.crawler = NationwideFuelPriceCrawler.__new__(NationwideFuelPriceCrawler)

    def test_domestic_prices_can_lead_crude_by_one_week(self):
        latest, previous = self.crawler._parse_combined(self.fuel, self.crude)

        self.assertEqual(latest.unleaded_95, 32.67)
        self.assertIsNone(latest.west_texas)
        self.assertIsNone(latest.dubai)
        self.assertIsNone(latest.brent)
        self.assertEqual(previous.west_texas, 103.54)
        self.assertEqual(previous.dubai, 120.47)
        self.assertEqual(previous.brent, 122.56)

    def test_unavailable_or_invalid_crude_does_not_discard_domestic_prices(self):
        for crude in (
            None,
            {"res": "error", "msg": "unavailable"},
            {"res": "01", "data": {}},
            {"res": "01", "data": {"crudeoil": [{"SurDate": "invalid"}]}},
        ):
            with self.subTest(crude=crude):
                records = self.crawler._parse_combined(self.fuel, crude)
                self.assertEqual(len(records), 2)
                self.assertEqual(records[0].unleaded_95, 32.67)
                self.assertTrue(all(item.west_texas is None for item in records))

    async def test_crude_http_or_json_failure_still_returns_domestic_history(self):
        for full_history in (False, True):
            for failure in ("timeout", "http", "json"):
                with self.subTest(full_history=full_history, failure=failure):
                    def respond(request):
                        if "/CrudeOil/" in request.url.path:
                            if failure == "timeout":
                                raise httpx.ReadTimeout("unavailable", request=request)
                            if failure == "http":
                                return httpx.Response(503)
                            return httpx.Response(200, text="not JSON")
                        if request.url.path.endswith("GetYearWeek"):
                            return httpx.Response(
                                200, json={"data": {"weeklist": [{"WeekId": 1}]}}
                            )
                        return httpx.Response(200, json=self.fuel)

                    async with httpx.AsyncClient(
                        transport=httpx.MockTransport(respond)
                    ) as client:
                        self.crawler.client = client
                        records = await self.crawler.fetch(full_history=full_history)
                    self.assertEqual(len(records), 2)
                    self.assertEqual(records[0].unleaded_95, 32.67)
                    self.assertIsNone(records[0].brent)

    async def test_domestic_http_failure_still_fails_the_sync(self):
        def respond(request):
            if "/Gasoline/" in request.url.path:
                return httpx.Response(503)
            return httpx.Response(200, json=self.crude)

        async with httpx.AsyncClient(transport=httpx.MockTransport(respond)) as client:
            self.crawler.client = client
            with self.assertRaises(httpx.HTTPStatusError):
                await self.crawler.fetch()

    @staticmethod
    def session_with(existing):
        return Mock(
            exec=AsyncMock(side_effect=[
                Mock(first=Mock(return_value=1 if existing else None)),
                Mock(first=Mock(return_value=1 if existing else None)),
                Mock(all=Mock(return_value=existing)),
            ]),
            commit=AsyncMock(),
        )

    async def test_sync_inserts_domestic_week_then_backfills_crude_without_duplicates(self):
        records = self.crawler._parse_combined(self.fuel, self.crude)
        self.crawler.fetch = AsyncMock(return_value=records)
        session = self.session_with([])

        self.assertEqual(await self.crawler.sync(session), 2)
        self.assertEqual(session.add.call_count, 2)
        session.commit.assert_awaited_once()
        self.assertIsNone(records[0].west_texas)

        self.crude["data"]["crudeoil"].insert(0, {
            "SurDate": "2026/09/20 ~ 2026/09/26",
            "WestT": 0.0,
            "Dubit": 120.0,
            "Burant": 122.0,
        })
        self.crawler.fetch.return_value = self.crawler._parse_combined(
            self.fuel, self.crude
        )
        session = self.session_with(records)

        self.assertEqual(await self.crawler.sync(session), 0)
        session.add.assert_not_called()
        session.commit.assert_awaited_once()
        self.assertEqual(records[0].west_texas, 0.0)
        self.assertEqual(records[0].dubai, 120.0)
        self.assertEqual(records[0].brent, 122.0)

    async def test_missing_crude_values_preserve_existing_prices(self):
        records = self.crawler._parse_combined(self.fuel, self.crude)
        self.fuel["data"]["gasoline"][1]["Oil95"] = 32.65
        self.crude["data"]["crudeoil"][0].update(
            WestT=None, Dubit=0.0, Burant=None
        )
        self.crawler.fetch = AsyncMock(return_value=self.crawler._parse_combined(
            self.fuel, self.crude
        ))
        session = self.session_with(records)

        self.assertEqual(await self.crawler.sync(session), 0)
        self.assertEqual(records[1].unleaded_95, 32.65)
        self.assertEqual(records[1].west_texas, 103.54)
        self.assertEqual(records[1].dubai, 0.0)
        self.assertEqual(records[1].brent, 122.56)
        session.add.assert_not_called()
        session.commit.assert_awaited_once()


if __name__ == "__main__":
    unittest.main()
