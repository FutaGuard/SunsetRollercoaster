import math
import re
from datetime import datetime, timedelta, timezone
from typing import Any

from loguru import logger
from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from sunsetRollercoaster.config import get_config
from sunsetRollercoaster.models.finance import FinanceExchangeRate

from ._crawler import Crawler

_CURRENCY_RE = re.compile(r"^[A-Z]{3}$")
# GOOGLEFINANCE 最多提供約 10 位有效數字，多出來的是浮點數表示誤差。
_SIGNIFICANT_DIGITS = 10


class FinanceExchangeRateCrawler(Crawler):
    """同步 Apps Script（apps-script/finance）以 GOOGLEFINANCE 取得的即時匯率。

    GOOGLEFINANCE 的歷史匯率無法透過 Apps Script 讀取，歷史資料靠定時同步累積。
    Web App 網址設定於 env_config.yml 的 `finance.exchange_rate_url`。
    """

    INTERVAL = timedelta(hours=1)

    def __init__(self, proxy: Crawler.Proxy = Crawler.Proxy.NO):
        super().__init__(proxy)

    async def query(self, url: str) -> dict[str, Any]:
        response = await self.client.get(
            url,
            headers={
                "Accept": "application/json",
                # Crawler 的共用 header 會宣告 br，但專案目前沒有 Brotli decoder。
                "Accept-Encoding": "gzip, deflate",
            },
        )
        response.raise_for_status()
        payload = response.json()
        if not isinstance(payload, dict):
            raise ValueError("exchange rate response is not a JSON object")
        return payload

    async def fetch(self) -> list[FinanceExchangeRate]:
        finance = get_config().finance
        if finance is None:
            raise ValueError("env_config.yml 未設定 finance.exchange_rate_url")
        return self._parse(await self.query(finance.exchange_rate_url))

    async def sync(self, session: AsyncSession) -> int:
        items = await self.fetch()
        observed_at = items[0].observed_at
        rows = (
            await session.exec(
                select(FinanceExchangeRate.base, FinanceExchangeRate.quote).where(
                    FinanceExchangeRate.observed_at == observed_at
                )
            )
        ).all()
        existing_pairs = {(base, quote) for base, quote in rows}

        added = 0
        for item in items:
            if (item.base, item.quote) in existing_pairs:
                continue
            session.add(item)
            added += 1

        await session.commit()
        return added

    @classmethod
    def _parse(cls, payload: dict[str, Any]) -> list[FinanceExchangeRate]:
        if "error" in payload:
            raise ValueError(f"exchange rate request failed: {payload['error']}")

        observed_at = cls._parse_timestamp(payload.get("fetched_at"))
        raw_items = payload.get("rates")
        if not isinstance(raw_items, list):
            raise ValueError("exchange rate response has no rates list")

        results: list[FinanceExchangeRate] = []
        for raw in raw_items:
            if not isinstance(raw, dict):
                raise ValueError("exchange rate item is not a JSON object")
            base = cls._currency(raw.get("base"), "base")
            quote = cls._currency(raw.get("quote"), "quote")
            rate = raw.get("rate")
            if rate is None:
                # 個別幣別暫時取不到時 Apps Script 回傳 null，下次同步再補。
                logger.warning("{}/{} 匯率暫時無法取得", base, quote)
                continue
            results.append(
                FinanceExchangeRate(
                    base=base,
                    quote=quote,
                    rate=cls._rate(rate, f"{base}{quote}"),
                    observed_at=observed_at,
                )
            )

        if not results:
            raise ValueError("exchange rate response has no available rate")
        return results

    @staticmethod
    def _parse_timestamp(value: Any) -> datetime:
        if not isinstance(value, str):
            raise ValueError(f"invalid exchange rate fetched_at: {value!r}")
        try:
            parsed = datetime.fromisoformat(value)
        except ValueError as exc:
            raise ValueError(f"invalid exchange rate fetched_at: {value!r}") from exc
        if parsed.tzinfo is None:
            raise ValueError(f"invalid exchange rate fetched_at: {value!r}")
        return parsed.astimezone(timezone.utc)

    @staticmethod
    def _currency(value: Any, field: str) -> str:
        if not isinstance(value, str) or _CURRENCY_RE.fullmatch(value) is None:
            raise ValueError(f"invalid exchange rate {field}: {value!r}")
        return value

    @staticmethod
    def _rate(value: Any, pair: str) -> float:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ValueError(f"invalid exchange rate {pair}: {value!r}")
        rate = float(value)
        if not math.isfinite(rate) or rate <= 0:
            raise ValueError(f"invalid exchange rate {pair}: {value!r}")
        return float(f"{rate:.{_SIGNIFICANT_DIGITS}g}")
