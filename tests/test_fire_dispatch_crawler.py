import unittest
from datetime import datetime, timezone

from sunsetRollercoaster.crawler.fire_dispatch import FireDispatchRecordCrawler


def county(code, cases, fetched_at="2026-11-27T02:00:05Z", error=None):
    return {
        "code": code,
        "name": code,
        "source_url": "https://example.gov.tw",
        "fetched_at": None if error else fetched_at,
        "error": error,
        "cases": cases,
    }


def case(**fields):
    return {
        "received_at": "2026-11-27T01:58:00Z",
        "category": "緊急救護-急病",
        "location": "臺南市北區",
        "unit": "和緯分隊",
        "status": "已出動",
        "case_id": None,
        "latitude": None,
        "longitude": None,
        **fields,
    }


class FireDispatchRecordParseTest(unittest.TestCase):
    def test_parse_stamps_each_case_with_its_county_fetch_time(self):
        fetched_at = datetime(2026, 11, 27, 2, 0, 5, tzinfo=timezone.utc)

        [record] = FireDispatchRecordCrawler._parse([county("tainan", [case(case_id="112138")])])

        self.assertEqual(
            (record.county, record.case_id, record.status, record.unit),
            ("tainan", "112138", "已出動", "和緯分隊"),
        )
        self.assertEqual(record.received_at, datetime(2026, 11, 27, 1, 58, tzinfo=timezone.utc))
        self.assertEqual((record.first_seen_at, record.last_seen_at), (fetched_at, fetched_at))

    def test_parse_skips_failed_counties_and_keeps_cases_without_receive_time(self):
        records = FireDispatchRecordCrawler._parse(
            [
                county("kaohsiung", [], error="fire dispatch source request failed"),
                county(
                    "new-taipei",
                    [case(received_at=None, status=None, case_id="261126211198", latitude=24.99, longitude=121.45)],
                ),
            ]
        )

        self.assertEqual(
            [(r.county, r.received_at, r.case_id, r.latitude) for r in records],
            [("new-taipei", None, "261126211198", 24.99)],
        )

    def test_parse_collapses_identical_rows_in_one_fetch(self):
        # 兩列完全相同時，同一批 upsert 不能重複同一個狀態。
        records = FireDispatchRecordCrawler._parse(
            [county("kaohsiung", [case(), case(), case(status="已到達")])]
        )

        self.assertEqual([r.status for r in records], ["已出動", "已到達"])

    def test_parse_rejects_malformed_payload(self):
        with self.assertRaisesRegex(ValueError, "invalid fire dispatch county code"):
            FireDispatchRecordCrawler._parse([{"cases": []}])
        with self.assertRaisesRegex(ValueError, "invalid fire dispatch fetched_at"):
            FireDispatchRecordCrawler._parse([county("tainan", [case()], fetched_at="2026-11-27T02:00:05")])


if __name__ == "__main__":
    unittest.main()
