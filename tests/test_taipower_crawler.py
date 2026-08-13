import unittest
from datetime import date, datetime, timedelta, timezone

from sunsetRollercoaster.crawler.taipower import (
    TaipowerAreaCrawler,
    TaipowerFuelMixCrawler,
    TaipowerGeneratorCrawler,
    TaipowerOperatingReserveCrawler,
    TaipowerPowerSnapshotCrawler,
)

_TW_TZ = timezone(timedelta(hours=8))


class TaipowerPowerSnapshotCrawlerTest(unittest.TestCase):
    def test_parse_power_snapshot(self):
        payload = {
            "success": "true",
            "records": [
                {"curr_load": "3879.6", "curr_util_rate": "79"},
                {
                    "fore_maxi_sply_capacity": "4878.1",
                    "fore_peak_dema_load": "4110.0",
                    "fore_peak_resv_capacity": "768.1",
                    "fore_peak_resv_rate": "18.69",
                    "fore_peak_resv_indicator": "G",
                    "fore_peak_hour_range": "13:00-16:00",
                    "publish_time": "115.08.13(四)11:00",
                },
                {
                    "yday_date": "115.08.12",
                    "yday_maxi_sply_capacity": "4729.8",
                    "yday_peak_dema_load": "4088.2",
                    "yday_peak_resv_capacity": "641.6",
                    "yday_peak_resv_rate": "15.69",
                    "yday_peak_resv_indicator": "G",
                },
                {
                    "real_hr_maxi_sply_capacity": "4879.4",
                    "real_hr_peak_time": "2026.08.13 02:54",
                },
            ],
        }

        item = TaipowerPowerSnapshotCrawler._parse(payload)

        self.assertEqual(item.published_at, datetime(2026, 8, 13, 11, tzinfo=_TW_TZ))
        self.assertEqual(item.current_load_mw, 38796.0)
        self.assertEqual(item.forecast_max_supply_mw, 48781.0)
        self.assertEqual(item.forecast_peak_reserve_mw, 7681.0)
        self.assertEqual(item.forecast_peak_reserve_rate_percent, 18.69)
        self.assertEqual(item.yesterday_date, date(2026, 8, 12))
        self.assertEqual(item.yesterday_peak_demand_mw, 40882.0)
        self.assertEqual(
            item.realtime_peak_at,
            datetime(2026, 8, 13, 2, 54, tzinfo=_TW_TZ),
        )

    def test_parse_rejects_unsuccessful_payload(self):
        with self.assertRaisesRegex(ValueError, "request failed"):
            TaipowerPowerSnapshotCrawler._parse({"success": "false", "records": []})


class TaipowerFuelMixCrawlerTest(unittest.TestCase):
    def test_parse_fuel_mix_and_convert_to_mw(self):
        raw = "00:00,1351.0,445.3,843.6,118.2,201.4,34.7,0.0,163.5,68.8,42.7,3.7,-0.8\n"

        items = TaipowerFuelMixCrawler._parse(raw, date(2026, 8, 13))

        self.assertEqual(len(items), 1)
        item = items[0]
        self.assertEqual(item.observed_at, datetime(2026, 8, 13, tzinfo=_TW_TZ))
        self.assertEqual(item.lng_mw, 13510.0)
        self.assertEqual(item.ipp_lng_mw, 4453.0)
        self.assertEqual(item.energy_storage_load_mw, -8.0)
        self.assertEqual(item.total_mw, 32721.0)

    def test_parse_rejects_changed_column_count(self):
        with self.assertRaisesRegex(ValueError, "expected 13 columns"):
            TaipowerFuelMixCrawler._parse(
                "00:00,1.0,2.0\n",
                date(2026, 8, 13),
            )


class TaipowerAreaCrawlerTest(unittest.TestCase):
    def test_parse_area_load_order_and_hour_only_time(self):
        items = TaipowerAreaCrawler._parse_loads(
            "00,50.2,1130.4,931.9,1159.6\n",
            date(2026, 8, 13),
        )

        item = items[0]
        self.assertEqual(item.observed_at, datetime(2026, 8, 13, tzinfo=_TW_TZ))
        self.assertEqual(item.east_load_mw, 502.0)
        self.assertEqual(item.south_load_mw, 11304.0)
        self.assertEqual(item.central_load_mw, 9319.0)
        self.assertEqual(item.north_load_mw, 11596.0)
        self.assertEqual(item.total_load_mw, 32721.0)

    def test_parse_area_generation_and_load_snapshot(self):
        items = TaipowerAreaCrawler._parse_snapshots(
            "2026-08-13 11:00,1265.2,1535.4,1191.9,1040.8,1398.9,1250.3,23.6,53.2\n"
        )

        item = items[0]
        self.assertEqual(
            item.observed_at,
            datetime(2026, 8, 13, 11, tzinfo=_TW_TZ),
        )
        self.assertEqual(item.north_generation_mw, 12652.0)
        self.assertEqual(item.north_load_mw, 15354.0)
        self.assertEqual(item.east_generation_mw, 236.0)
        self.assertEqual(item.east_load_mw, 532.0)


class TaipowerOperatingReserveCrawlerTest(unittest.TestCase):
    def test_parse_history_skips_unpublished_calendar_rows(self):
        raw = "01/01,2729.8,365.3,13.38\n08/13,,,\n,,,"

        items = TaipowerOperatingReserveCrawler._parse_history(raw, 2026)

        self.assertEqual(len(items), 1)
        self.assertEqual(items[0].date, date(2026, 1, 1))
        self.assertEqual(items[0].peak_load_mw, 27298.0)
        self.assertEqual(items[0].reserve_capacity_mw, 3653.0)
        self.assertEqual(items[0].reserve_rate_percent, 13.38)
        self.assertFalse(items[0].is_forecast)

    def test_parse_current_year_can_be_empty_before_first_actual(self):
        items = TaipowerOperatingReserveCrawler._parse_history(
            "01/01,,,\n,,,,",
            2027,
            allow_empty=True,
        )

        self.assertEqual(items, [])

    def test_archive_backfill_retries_partial_and_forecast_years(self):
        existing = [
            (date(2023, 1, 1), False),
            *(
                (date(2024, 1, 1) + timedelta(days=offset), False)
                for offset in range(366)
            ),
            *(
                (date(2025, 1, 1) + timedelta(days=offset), False)
                for offset in range(364)
            ),
            (date(2025, 12, 31), True),
        ]

        missing = TaipowerOperatingReserveCrawler._archive_years_to_fetch(
            existing,
            2026,
        )

        self.assertIn(2023, missing)
        self.assertNotIn(2024, missing)
        self.assertIn(2025, missing)

    def test_parse_today_forecast_from_loadpara_text(self):
        raw = """
var loadInfo = [
"38,796.0",
"41,100.0",
"48,781.0",
"115.08.13(四)11:00更新"
];
"""

        item = TaipowerOperatingReserveCrawler._parse_forecast(raw)

        self.assertEqual(item.date, date(2026, 8, 13))
        self.assertEqual(item.peak_load_mw, 41100.0)
        self.assertEqual(item.reserve_capacity_mw, 7681.0)
        self.assertAlmostEqual(item.reserve_rate_percent, 18.6885644769)
        self.assertTrue(item.is_forecast)
        self.assertEqual(
            item.published_at,
            datetime(2026, 8, 13, 11, tzinfo=_TW_TZ),
        )


class TaipowerGeneratorCrawlerTest(unittest.TestCase):
    def test_parse_generator_units_summaries_and_missing_values(self):
        payload = {
            "": "2026-08-13 10:50",
            "aaData": [
                [
                    "<A NAME='lng'></A><b>燃氣(LNG)</b>",
                    "",
                    "大潭CC#1",
                    "742.7",
                    "388.1",
                    "52.255%",
                    "運轉限制",
                    "",
                ],
                [
                    "<A NAME='lng'></A><b>燃氣(LNG)</b>",
                    "",
                    "小計",
                    "15918.1(26.264%)",
                    "14232.0(35.658%)",
                    "",
                    " ",
                    "",
                ],
                [
                    "<A NAME='fueloil'></A><b>燃料油(Fuel Oil)</b>",
                    "輕油(Diesel)",
                    "離島其它(註4)",
                    "44.2",
                    "N/A",
                    "N/A",
                    " ",
                    "",
                ],
            ],
            "SubUnitSet": [],
        }

        items = TaipowerGeneratorCrawler._parse(payload)

        self.assertEqual(len(items), 3)
        self.assertEqual(
            items[0].published_at,
            datetime(2026, 8, 13, 10, 50, tzinfo=_TW_TZ),
        )
        self.assertEqual(items[0].category_code, "lng")
        self.assertEqual(items[0].category, "燃氣(LNG)")
        self.assertEqual(items[0].installed_capacity_mw, 742.7)
        self.assertEqual(items[0].net_generation_mw, 388.1)
        self.assertEqual(items[0].utilization_percent, 52.255)
        self.assertEqual(items[0].status, "運轉限制")
        self.assertTrue(items[1].is_summary)
        self.assertEqual(items[1].installed_capacity_percent, 26.264)
        self.assertEqual(items[1].net_generation_percent, 35.658)
        self.assertIsNone(items[2].net_generation_mw)
        self.assertIsNone(items[2].utilization_percent)


if __name__ == "__main__":
    unittest.main()
