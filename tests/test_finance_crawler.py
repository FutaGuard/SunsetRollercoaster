import unittest
from datetime import datetime, timezone
from unittest.mock import AsyncMock, Mock

from sunsetRollercoaster.crawler.finance import FinanceExchangeRateCrawler


def payload(**rates):
    return {
        "quote": "TWD",
        "fetched_at": "2026-10-20T03:06:33.294Z",
        "rates": [
            {"base": base, "quote": "TWD", "rate": rate}
            for base, rate in rates.items()
        ],
    }


class FinanceExchangeRateParseTest(unittest.TestCase):
    def test_parse_rates_at_utc_fetch_time(self):
        result = FinanceExchangeRateCrawler._parse(
            payload(USD=32.5885, CHF=39.508520000000004, IDR=0.0018100820000000002)
        )

        self.assertEqual(
            [(item.base, item.quote, item.rate) for item in result],
            [
                ("USD", "TWD", 32.5885),
                ("CHF", "TWD", 39.50852),
                ("IDR", "TWD", 0.001810082),
            ],
        )
        expected_at = datetime(2026, 10, 20, 3, 6, 33, 294000, tzinfo=timezone.utc)
        self.assertTrue(all(item.observed_at == expected_at for item in result))

    def test_parse_skips_currencies_without_rate(self):
        result = FinanceExchangeRateCrawler._parse(payload(USD=32.5885, VND=None))

        self.assertEqual([item.base for item in result], ["USD"])

    def test_parse_rejects_response_without_any_rate(self):
        with self.assertRaisesRegex(ValueError, "no available rate"):
            FinanceExchangeRateCrawler._parse(payload(USD=None, JPY=None))

    def test_parse_rejects_apps_script_error(self):
        with self.assertRaisesRegex(ValueError, "request failed: Lock timeout"):
            FinanceExchangeRateCrawler._parse({"error": "Lock timeout"})

    def test_parse_rejects_invalid_rate(self):
        for rate in (0, -1.5, "32.5", True, float("nan")):
            with self.subTest(rate=rate):
                with self.assertRaisesRegex(ValueError, "invalid exchange rate USDTWD"):
                    FinanceExchangeRateCrawler._parse(payload(USD=rate))

    def test_parse_rejects_invalid_currency_or_timestamp(self):
        bad_currency = payload(USD=32.5)
        bad_currency["rates"][0]["base"] = "usd"
        naive_time = payload(USD=32.5)
        naive_time["fetched_at"] = "2026-10-20T03:06:33"

        with self.assertRaisesRegex(ValueError, "invalid exchange rate base"):
            FinanceExchangeRateCrawler._parse(bad_currency)
        with self.assertRaisesRegex(ValueError, "invalid exchange rate fetched_at"):
            FinanceExchangeRateCrawler._parse(naive_time)


class FinanceExchangeRateSyncTest(unittest.IsolatedAsyncioTestCase):
    async def test_sync_inserts_only_pairs_missing_at_same_fetch_time(self):
        crawler = FinanceExchangeRateCrawler.__new__(FinanceExchangeRateCrawler)
        crawler.fetch = AsyncMock(
            return_value=FinanceExchangeRateCrawler._parse(
                payload(USD=32.5885, JPY=0.2056809, EUR=36.61262)
            )
        )
        session = Mock(
            exec=AsyncMock(return_value=Mock(all=Mock(return_value=[("USD", "TWD")]))),
            commit=AsyncMock(),
        )

        self.assertEqual(await crawler.sync(session), 2)
        self.assertEqual(
            [call.args[0].base for call in session.add.call_args_list],
            ["JPY", "EUR"],
        )
        session.commit.assert_awaited_once()


if __name__ == "__main__":
    unittest.main()
