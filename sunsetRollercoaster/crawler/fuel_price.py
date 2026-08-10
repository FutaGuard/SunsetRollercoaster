import math
import re
from datetime import datetime, timedelta, timezone
from typing import Any

from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from sunsetRollercoaster.models.fuel_price import NationwideFuelPrice

from ._crawler import Crawler

_WEEK_PERIOD_RE = re.compile(
    r"^\s*(\d{4}/\d{2}/\d{2})\s*~\s*(\d{4}/\d{2}/\d{2})\s*$"
)
_TW_TZ = timezone(timedelta(hours=8))


class NationwideFuelPriceCrawler(Crawler):
    """同步經濟部能源署公布的全國汽柴油週均價。"""

    INTERVAL = timedelta(days=1)
    LOAD_URL = "https://www2.moeaea.gov.tw/oil111/Gasoline/NationwideAvg/load"
    RANGE_URL = "https://www2.moeaea.gov.tw/oil111/Gasoline/GetRangeNationwideAvg"
    YEAR_WEEK_URL = "https://www2.moeaea.gov.tw/oil111/Common/GetYearWeek"
    UNIT = "week"
    FIRST_DATA_YEAR = 2003

    def __init__(self, proxy: Crawler.Proxy = Crawler.Proxy.NO):
        super().__init__(proxy)

    async def query(self, url: str) -> dict[str, Any]:
        return await self._post_json(url, {"unit": self.UNIT})

    async def sync(self, session: AsyncSession) -> int:
        has_data = (
            await session.exec(select(NationwideFuelPrice.id).limit(1))
        ).first() is not None
        items = await self.fetch(full_history=not has_data)

        if not items:
            return 0

        period_starts = [item.period_start for item in items]
        existing_items = (
            await session.exec(
                select(NationwideFuelPrice).where(
                    NationwideFuelPrice.period_start.in_(period_starts)
                )
            )
        ).all()
        existing_by_period = {
            (item.period_start, item.period_end): item for item in existing_items
        }

        added = 0
        for item in items:
            existing = existing_by_period.get((item.period_start, item.period_end))
            if existing is None:
                session.add(item)
                added += 1
                continue

            existing.unleaded_92 = item.unleaded_92
            existing.unleaded_95 = item.unleaded_95
            existing.unleaded_98 = item.unleaded_98
            existing.super_diesel = item.super_diesel

        await session.commit()
        return added

    async def fetch(self, full_history: bool = False) -> list[NationwideFuelPrice]:
        """取得最近 20 週，或首次同步所需的完整歷史週資料。"""

        latest_payload = await self.query(self.LOAD_URL)
        if not full_history:
            return self._parse(latest_payload)

        latest_data = self._response_data(latest_payload)
        end_week = self._positive_int(latest_data.get("endweek"), "endweek")
        start_week = await self._first_week_id()
        history_payload = await self._post_json(
            self.RANGE_URL,
            {
                "unit": self.UNIT,
                "start": str(start_week),
                "end": str(end_week),
            },
        )
        return self._parse(history_payload)

    async def _first_week_id(self) -> int:
        payload = await self._post_json(
            self.YEAR_WEEK_URL,
            {"year": str(self.FIRST_DATA_YEAR)},
        )
        data = payload.get("data")
        weeklist = data.get("weeklist") if isinstance(data, dict) else None
        if not isinstance(weeklist, list) or not weeklist:
            raise ValueError("fuel price response has no week list")

        week_ids = [
            self._positive_int(item.get("WeekId"), "WeekId")
            for item in weeklist
            if isinstance(item, dict)
        ]
        if not week_ids:
            raise ValueError("fuel price response has no valid week id")
        return min(week_ids)

    async def _post_json(self, url: str, data: dict[str, str]) -> dict[str, Any]:
        response = await self.client.post(
            url,
            data=data,
            headers={
                "Accept": "application/json",
                # Crawler 的共用 header 會宣告 br，但專案目前沒有 Brotli decoder。
                "Accept-Encoding": "gzip, deflate",
            },
        )
        response.raise_for_status()
        payload = response.json()
        if not isinstance(payload, dict):
            raise ValueError("fuel price response is not a JSON object")
        return payload

    @classmethod
    def _parse(cls, payload: dict[str, Any]) -> list[NationwideFuelPrice]:
        data = cls._response_data(payload)
        raw_items = data.get("gasoline")
        if not isinstance(raw_items, list):
            raise ValueError("fuel price response has no gasoline list")

        results: list[NationwideFuelPrice] = []
        for raw in raw_items:
            if not isinstance(raw, dict):
                raise ValueError("fuel price item is not a JSON object")
            period_start, period_end = cls._parse_week_period(raw.get("SurDate"))
            results.append(
                NationwideFuelPrice(
                    period_start=period_start,
                    period_end=period_end,
                    unleaded_92=cls._price(raw.get("Oil92"), "Oil92"),
                    unleaded_95=cls._price(raw.get("Oil95"), "Oil95"),
                    unleaded_98=cls._price(raw.get("Oil98"), "Oil98"),
                    super_diesel=cls._price(raw.get("Oilchai"), "Oilchai"),
                )
            )
        return results

    @staticmethod
    def _response_data(payload: dict[str, Any]) -> dict[str, Any]:
        if payload.get("res") != "01":
            raise ValueError(f"fuel price request failed: {payload.get('msg') or 'unknown error'}")
        data = payload.get("data")
        if not isinstance(data, dict):
            raise ValueError("fuel price response has no data object")
        return data

    @staticmethod
    def _parse_week_period(value: Any) -> tuple[datetime, datetime]:
        match = _WEEK_PERIOD_RE.fullmatch(str(value or ""))
        if match is None:
            raise ValueError(f"invalid fuel price period: {value!r}")
        period_start = datetime.fromisoformat(
            match.group(1).replace("/", "-")
        ).replace(tzinfo=_TW_TZ)
        period_end = datetime.fromisoformat(
            match.group(2).replace("/", "-")
        ).replace(tzinfo=_TW_TZ)
        if period_start > period_end:
            raise ValueError(f"invalid fuel price period: {value!r}")
        return period_start, period_end

    @staticmethod
    def _price(value: Any, field: str) -> float:
        if isinstance(value, bool):
            raise ValueError(f"invalid fuel price {field}: {value!r}")
        try:
            price = float(value)
        except (TypeError, ValueError) as exc:
            raise ValueError(f"invalid fuel price {field}: {value!r}") from exc
        if not math.isfinite(price) or price < 0:
            raise ValueError(f"invalid fuel price {field}: {value!r}")
        return price

    @staticmethod
    def _positive_int(value: Any, field: str) -> int:
        if isinstance(value, bool):
            raise ValueError(f"invalid fuel price {field}: {value!r}")
        try:
            result = int(value)
        except (TypeError, ValueError) as exc:
            raise ValueError(f"invalid fuel price {field}: {value!r}") from exc
        if result <= 0:
            raise ValueError(f"invalid fuel price {field}: {value!r}")
        return result
