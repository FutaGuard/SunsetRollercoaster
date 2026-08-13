import asyncio
import csv
import html
import io
import math
import re
from datetime import date, datetime, time, timedelta, timezone
from email.utils import parsedate_to_datetime
from typing import Any, Iterable

import httpx
from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from sunsetRollercoaster.models.taipower import (
    TaipowerAreaLoad,
    TaipowerAreaSnapshot,
    TaipowerFuelMix,
    TaipowerGenerator,
    TaipowerOperatingReserve,
    TaipowerPowerSnapshot,
)

from ._crawler import Crawler

_TW_TZ = timezone(timedelta(hours=8))
_POWER_CURVE_TO_MW = 10.0  # 官方曲線資料的單位為「萬瓩」，即 10 MW。
_ROC_DATE_RE = re.compile(
    r"^\s*(?P<year>\d{2,3})[./-](?P<month>\d{1,2})[./-](?P<day>\d{1,2})\s*$"
)
_ROC_DATETIME_RE = re.compile(
    r"^\s*(?P<year>\d{2,3})[./-](?P<month>\d{1,2})[./-](?P<day>\d{1,2})"
    r"(?:\([^)]*\))?\s*(?P<hour>\d{1,2}):(?P<minute>\d{1,2})\s*(?:更新)?\s*$"
)
_CATEGORY_CODE_RE = re.compile(r"\bNAME=['\"]([^'\"]+)['\"]", re.IGNORECASE)
_HTML_TAG_RE = re.compile(r"<[^>]+>")
_VALUE_WITH_PERCENT_RE = re.compile(
    r"^\s*(?P<value>-?[\d,]+(?:\.\d+)?)"
    r"(?:\s*\(\s*(?P<percent>-?[\d,]+(?:\.\d+)?)%\s*\))?\s*$"
)
_LOAD_INFO_RE = re.compile(
    r"var\s+loadInfo\s*=\s*\[(?P<values>.*?)\]\s*;",
    re.DOTALL,
)


def _optional_text(value: Any) -> str | None:
    if value is None:
        return None
    text = html.unescape(str(value)).strip()
    return None if text in {"", "-", "N/A"} else text


def _number(value: Any, field: str, *, optional: bool = False) -> float | None:
    text = _optional_text(value)
    if text is None:
        if optional:
            return None
        raise ValueError(f"missing Taipower value: {field}")
    if isinstance(value, bool):
        raise ValueError(f"invalid Taipower value {field}: {value!r}")
    try:
        result = float(text.replace(",", ""))
    except ValueError as exc:
        raise ValueError(f"invalid Taipower value {field}: {value!r}") from exc
    if not math.isfinite(result):
        raise ValueError(f"invalid Taipower value {field}: {value!r}")
    return result


def _mw(value: Any, field: str, *, optional: bool = False) -> float | None:
    result = _number(value, field, optional=optional)
    return None if result is None else result * _POWER_CURVE_TO_MW


def _parse_roc_date(value: Any, field: str) -> date:
    match = _ROC_DATE_RE.fullmatch(str(value or ""))
    if match is None:
        raise ValueError(f"invalid Taipower {field}: {value!r}")
    try:
        return date(
            int(match.group("year")) + 1911,
            int(match.group("month")),
            int(match.group("day")),
        )
    except ValueError as exc:
        raise ValueError(f"invalid Taipower {field}: {value!r}") from exc


def _parse_roc_datetime(value: Any, field: str) -> datetime:
    match = _ROC_DATETIME_RE.fullmatch(str(value or ""))
    if match is None:
        raise ValueError(f"invalid Taipower {field}: {value!r}")
    try:
        return datetime(
            int(match.group("year")) + 1911,
            int(match.group("month")),
            int(match.group("day")),
            int(match.group("hour")),
            int(match.group("minute")),
            tzinfo=_TW_TZ,
        )
    except ValueError as exc:
        raise ValueError(f"invalid Taipower {field}: {value!r}") from exc


def _parse_local_datetime(value: Any, field: str) -> datetime:
    raw = str(value or "").strip()
    date_match = re.match(
        r"^(?P<year>\d{4})[./-](?P<month>\d{1,2})[./-](?P<day>\d{1,2})(?P<rest>.*)$",
        raw,
    )
    if date_match is not None:
        raw = (
            f"{int(date_match.group('year')):04d}-"
            f"{int(date_match.group('month')):02d}-"
            f"{int(date_match.group('day')):02d}"
            f"{date_match.group('rest')}"
        )
    try:
        parsed = datetime.fromisoformat(raw)
    except ValueError as exc:
        raise ValueError(f"invalid Taipower {field}: {value!r}") from exc
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=_TW_TZ)
    return parsed.astimezone(_TW_TZ)


def _parse_curve_time(value: Any, observed_date: date, field: str) -> datetime:
    raw = str(value or "").strip()
    if re.fullmatch(r"\d{2}", raw):
        raw += ":00"
    match = re.fullmatch(r"(?P<hour>\d{2}):(?P<minute>\d{2})", raw)
    if match is None:
        raise ValueError(f"invalid Taipower {field}: {value!r}")
    try:
        observed_time = time(int(match.group("hour")), int(match.group("minute")))
    except ValueError as exc:
        raise ValueError(f"invalid Taipower {field}: {value!r}") from exc
    return datetime.combine(observed_date, observed_time, tzinfo=_TW_TZ)


def _response_date(response: httpx.Response) -> date:
    last_modified = response.headers.get("Last-Modified")
    if last_modified:
        try:
            parsed = parsedate_to_datetime(last_modified)
            if parsed.tzinfo is None:
                parsed = parsed.replace(tzinfo=timezone.utc)
            return parsed.astimezone(_TW_TZ).date()
        except (TypeError, ValueError, OverflowError):
            pass
    return datetime.now(_TW_TZ).date()


def _update_model(target: Any, source: Any) -> None:
    for field in source.__class__.model_fields:
        if field != "id":
            setattr(target, field, getattr(source, field))


class _TaipowerCrawler(Crawler):
    BASE_URL = "https://www.taipower.com.tw/d006/loadGraph/loadGraph"
    REFERER = f"{BASE_URL}/load_fueltype_.html"

    async def _get_response(self, url: str) -> httpx.Response:
        response = await self.client.get(
            url,
            headers={
                "Accept": "application/json,text/plain,*/*",
                "Accept-Encoding": "gzip, deflate",
                "Referer": self.REFERER,
            },
        )
        response.raise_for_status()
        return response

    async def query(self, url: str) -> str:
        return (await self._get_response(url)).text


class TaipowerPowerSnapshotCrawler(_TaipowerCrawler):
    """同步台電「今日電力資訊」。"""

    INTERVAL = timedelta(minutes=10)
    URL = f"{_TaipowerCrawler.BASE_URL}/data/loadpara.json"

    async def fetch(self) -> TaipowerPowerSnapshot:
        response = await self._get_response(self.URL)
        payload = response.json()
        if not isinstance(payload, dict):
            raise ValueError("Taipower power snapshot is not a JSON object")
        return self._parse(payload)

    @classmethod
    def _parse(cls, payload: dict[str, Any]) -> TaipowerPowerSnapshot:
        if str(payload.get("success", "")).lower() != "true":
            raise ValueError("Taipower power snapshot request failed")
        records = payload.get("records")
        if not isinstance(records, list) or not records:
            raise ValueError("Taipower power snapshot has no records")

        values: dict[str, Any] = {}
        for record in records:
            if not isinstance(record, dict):
                raise ValueError("Taipower power snapshot record is not an object")
            values.update(record)

        published_at = _parse_roc_datetime(values.get("publish_time"), "publish_time")
        realtime_peak_at = values.get("real_hr_peak_time")
        yesterday_date = values.get("yday_date")
        return TaipowerPowerSnapshot(
            published_at=published_at,
            current_load_mw=_mw(values.get("curr_load"), "curr_load", optional=True),
            current_utilization_percent=_number(
                values.get("curr_util_rate"),
                "curr_util_rate",
                optional=True,
            ),
            forecast_max_supply_mw=_mw(
                values.get("fore_maxi_sply_capacity"),
                "fore_maxi_sply_capacity",
                optional=True,
            ),
            forecast_peak_demand_mw=_mw(
                values.get("fore_peak_dema_load"),
                "fore_peak_dema_load",
                optional=True,
            ),
            forecast_peak_reserve_mw=_mw(
                values.get("fore_peak_resv_capacity"),
                "fore_peak_resv_capacity",
                optional=True,
            ),
            forecast_peak_reserve_rate_percent=_number(
                values.get("fore_peak_resv_rate"),
                "fore_peak_resv_rate",
                optional=True,
            ),
            forecast_peak_reserve_indicator=_optional_text(
                values.get("fore_peak_resv_indicator")
            ),
            forecast_peak_hour_range=_optional_text(values.get("fore_peak_hour_range")),
            yesterday_date=(
                _parse_roc_date(yesterday_date, "yday_date")
                if _optional_text(yesterday_date) is not None
                else None
            ),
            yesterday_max_supply_mw=_mw(
                values.get("yday_maxi_sply_capacity"),
                "yday_maxi_sply_capacity",
                optional=True,
            ),
            yesterday_peak_demand_mw=_mw(
                values.get("yday_peak_dema_load"),
                "yday_peak_dema_load",
                optional=True,
            ),
            yesterday_peak_reserve_mw=_mw(
                values.get("yday_peak_resv_capacity"),
                "yday_peak_resv_capacity",
                optional=True,
            ),
            yesterday_peak_reserve_rate_percent=_number(
                values.get("yday_peak_resv_rate"),
                "yday_peak_resv_rate",
                optional=True,
            ),
            yesterday_peak_reserve_indicator=_optional_text(
                values.get("yday_peak_resv_indicator")
            ),
            realtime_max_supply_mw=_mw(
                values.get("real_hr_maxi_sply_capacity"),
                "real_hr_maxi_sply_capacity",
                optional=True,
            ),
            realtime_peak_at=(
                _parse_local_datetime(realtime_peak_at, "real_hr_peak_time")
                if _optional_text(realtime_peak_at) is not None
                else None
            ),
        )

    async def sync(self, session: AsyncSession) -> int:
        item = await self.fetch()
        existing = (
            await session.exec(
                select(TaipowerPowerSnapshot).where(
                    TaipowerPowerSnapshot.published_at == item.published_at
                )
            )
        ).first()
        if existing is None:
            session.add(item)
            added = 1
        else:
            _update_model(existing, item)
            added = 0
        await session.commit()
        return added


class TaipowerFuelMixCrawler(_TaipowerCrawler):
    """同步台電今日用電曲線（依燃料類別）。"""

    INTERVAL = timedelta(minutes=10)
    URL = f"{_TaipowerCrawler.BASE_URL}/data/loadfueltype.csv"

    async def fetch(self) -> list[TaipowerFuelMix]:
        response = await self._get_response(self.URL)
        return self._parse(response.text, _response_date(response))

    @classmethod
    def _parse(cls, raw: str, observed_date: date) -> list[TaipowerFuelMix]:
        results: list[TaipowerFuelMix] = []
        seen: set[datetime] = set()
        for line_number, row in enumerate(csv.reader(io.StringIO(raw)), start=1):
            if not row or not any(value.strip() for value in row):
                continue
            if len(row) != 13:
                raise ValueError(
                    f"invalid Taipower fuel mix row {line_number}: expected 13 columns"
                )
            observed_at = _parse_curve_time(row[0], observed_date, "fuel mix time")
            if observed_at in seen:
                raise ValueError(f"duplicate Taipower fuel mix time: {row[0]!r}")
            seen.add(observed_at)
            powers = [
                _mw(value, f"fuel mix column {index}")
                for index, value in enumerate(row[1:], start=1)
            ]
            results.append(
                TaipowerFuelMix(
                    observed_at=observed_at,
                    lng_mw=powers[0],
                    ipp_lng_mw=powers[1],
                    coal_mw=powers[2],
                    ipp_coal_mw=powers[3],
                    cogeneration_mw=powers[4],
                    fuel_oil_mw=powers[5],
                    solar_mw=powers[6],
                    wind_mw=powers[7],
                    hydro_mw=powers[8],
                    energy_storage_mw=powers[9],
                    other_renewable_mw=powers[10],
                    energy_storage_load_mw=powers[11],
                    total_mw=sum(powers),
                )
            )
        if not results:
            raise ValueError("Taipower fuel mix response has no data")
        return results

    async def sync(self, session: AsyncSession) -> int:
        items = await self.fetch()
        timestamps = [item.observed_at for item in items]
        existing_items = (
            await session.exec(
                select(TaipowerFuelMix).where(
                    TaipowerFuelMix.observed_at.in_(timestamps)
                )
            )
        ).all()
        existing_by_time = {item.observed_at: item for item in existing_items}
        added = 0
        for item in items:
            existing = existing_by_time.get(item.observed_at)
            if existing is None:
                session.add(item)
                added += 1
            else:
                _update_model(existing, item)
        await session.commit()
        return added


class TaipowerAreaCrawler(_TaipowerCrawler):
    """同步台電今日區域用電曲線與各區發用電快照。"""

    INTERVAL = timedelta(minutes=10)
    LOAD_URL = f"{_TaipowerCrawler.BASE_URL}/data/loadareas.csv"
    SNAPSHOT_URL = f"{_TaipowerCrawler.BASE_URL}/data/genloadareaperc.csv"

    async def fetch(
        self,
    ) -> tuple[list[TaipowerAreaLoad], list[TaipowerAreaSnapshot]]:
        load_response, snapshot_response = await asyncio.gather(
            self._get_response(self.LOAD_URL),
            self._get_response(self.SNAPSHOT_URL),
        )
        return (
            self._parse_loads(load_response.text, _response_date(load_response)),
            self._parse_snapshots(snapshot_response.text),
        )

    @classmethod
    def _parse_loads(cls, raw: str, observed_date: date) -> list[TaipowerAreaLoad]:
        results: list[TaipowerAreaLoad] = []
        seen: set[datetime] = set()
        for line_number, row in enumerate(csv.reader(io.StringIO(raw)), start=1):
            if not row or not any(value.strip() for value in row):
                continue
            if len(row) != 5:
                raise ValueError(
                    f"invalid Taipower area load row {line_number}: expected 5 columns"
                )
            observed_at = _parse_curve_time(row[0], observed_date, "area load time")
            if observed_at in seen:
                raise ValueError(f"duplicate Taipower area load time: {row[0]!r}")
            seen.add(observed_at)
            # 官方順序：東、南、中、北。
            east, south, central, north = [
                _mw(value, f"area load column {index}")
                for index, value in enumerate(row[1:], start=1)
            ]
            results.append(
                TaipowerAreaLoad(
                    observed_at=observed_at,
                    north_load_mw=north,
                    central_load_mw=central,
                    south_load_mw=south,
                    east_load_mw=east,
                    total_load_mw=north + central + south + east,
                )
            )
        if not results:
            raise ValueError("Taipower area load response has no data")
        return results

    @classmethod
    def _parse_snapshots(cls, raw: str) -> list[TaipowerAreaSnapshot]:
        results: list[TaipowerAreaSnapshot] = []
        seen: set[datetime] = set()
        for line_number, row in enumerate(csv.reader(io.StringIO(raw)), start=1):
            if not row or not any(value.strip() for value in row):
                continue
            if len(row) != 9:
                raise ValueError(
                    f"invalid Taipower area snapshot row {line_number}: "
                    "expected 9 columns"
                )
            observed_at = _parse_local_datetime(row[0], "area snapshot time")
            if observed_at in seen:
                raise ValueError(f"duplicate Taipower area snapshot time: {row[0]!r}")
            seen.add(observed_at)
            powers = [
                _mw(value, f"area snapshot column {index}")
                for index, value in enumerate(row[1:], start=1)
            ]
            results.append(
                TaipowerAreaSnapshot(
                    observed_at=observed_at,
                    north_generation_mw=powers[0],
                    north_load_mw=powers[1],
                    central_generation_mw=powers[2],
                    central_load_mw=powers[3],
                    south_generation_mw=powers[4],
                    south_load_mw=powers[5],
                    east_generation_mw=powers[6],
                    east_load_mw=powers[7],
                )
            )
        if not results:
            raise ValueError("Taipower area snapshot response has no data")
        return results

    async def sync(self, session: AsyncSession) -> int:
        loads, snapshots = await self.fetch()
        added = 0

        load_times = [item.observed_at for item in loads]
        existing_loads = (
            await session.exec(
                select(TaipowerAreaLoad).where(
                    TaipowerAreaLoad.observed_at.in_(load_times)
                )
            )
        ).all()
        existing_loads_by_time = {item.observed_at: item for item in existing_loads}
        for item in loads:
            existing = existing_loads_by_time.get(item.observed_at)
            if existing is None:
                session.add(item)
                added += 1
            else:
                _update_model(existing, item)

        snapshot_times = [item.observed_at for item in snapshots]
        existing_snapshots = (
            await session.exec(
                select(TaipowerAreaSnapshot).where(
                    TaipowerAreaSnapshot.observed_at.in_(snapshot_times)
                )
            )
        ).all()
        existing_snapshots_by_time = {
            item.observed_at: item for item in existing_snapshots
        }
        for item in snapshots:
            existing = existing_snapshots_by_time.get(item.observed_at)
            if existing is None:
                session.add(item)
                added += 1
            else:
                _update_model(existing, item)

        await session.commit()
        return added


class TaipowerOperatingReserveCrawler(_TaipowerCrawler):
    """同步每日尖峰負載、備轉容量及今日預估值。"""

    INTERVAL = timedelta(hours=1)
    FIRST_ARCHIVE_YEAR = 2018
    CURRENT_URL = f"{_TaipowerCrawler.BASE_URL}/data/reserve.csv"
    LOAD_INFO_URL = f"{_TaipowerCrawler.BASE_URL}/data/loadpara.txt"

    @classmethod
    def archive_url(cls, year: int) -> str:
        return f"{cls.BASE_URL}/data/reserve{year}.csv"

    async def fetch(
        self,
        archive_years: Iterable[int] = (),
        current_year: int | None = None,
    ) -> list[TaipowerOperatingReserve]:
        year = current_year or datetime.now(_TW_TZ).year
        years = sorted(set(archive_years))
        payloads = await asyncio.gather(
            self.query(self.CURRENT_URL),
            self.query(self.LOAD_INFO_URL),
            *(self.query(self.archive_url(archive_year)) for archive_year in years),
        )
        results = self._parse_history(payloads[0], year, allow_empty=True)
        for archive_year, payload in zip(years, payloads[2:]):
            results.extend(self._parse_history(payload, archive_year))

        by_date = {item.date: item for item in results}
        forecast = self._parse_forecast(payloads[1])
        # 當日實績若已公布，優先於 loadpara.txt 的預估值。
        by_date.setdefault(forecast.date, forecast)
        return [by_date[item_date] for item_date in sorted(by_date)]

    @classmethod
    def _parse_history(
        cls,
        raw: str,
        year: int,
        *,
        allow_empty: bool = False,
    ) -> list[TaipowerOperatingReserve]:
        results: list[TaipowerOperatingReserve] = []
        seen: set[date] = set()
        for line_number, row in enumerate(csv.reader(io.StringIO(raw)), start=1):
            if not row or not any(value.strip() for value in row):
                continue
            if len(row) != 4:
                raise ValueError(
                    f"invalid Taipower reserve row {line_number}: expected 4 columns"
                )
            if not any(value.strip() for value in row[1:]):
                continue
            match = re.fullmatch(r"(?P<month>\d{2})/(?P<day>\d{2})", row[0].strip())
            if match is None:
                raise ValueError(f"invalid Taipower reserve date: {row[0]!r}")
            try:
                item_date = date(
                    year, int(match.group("month")), int(match.group("day"))
                )
            except ValueError as exc:
                raise ValueError(f"invalid Taipower reserve date: {row[0]!r}") from exc
            if item_date in seen:
                raise ValueError(f"duplicate Taipower reserve date: {item_date}")
            seen.add(item_date)
            results.append(
                TaipowerOperatingReserve(
                    date=item_date,
                    peak_load_mw=_mw(row[1], "reserve peak load"),
                    reserve_capacity_mw=_mw(row[2], "reserve capacity"),
                    reserve_rate_percent=_number(row[3], "reserve rate"),
                    is_forecast=False,
                )
            )
        if not results and not allow_empty:
            raise ValueError(f"Taipower reserve response for {year} has no data")
        return results

    @classmethod
    def _parse_forecast(cls, raw: str) -> TaipowerOperatingReserve:
        match = _LOAD_INFO_RE.search(raw)
        if match is None:
            raise ValueError("Taipower loadpara.txt has no loadInfo array")
        values = re.findall(r'"([^"]*)"', match.group("values"))
        if len(values) != 4:
            raise ValueError("Taipower loadpara.txt loadInfo must contain four values")
        peak_load_mw = _number(values[1], "forecast peak load")
        max_supply_mw = _number(values[2], "forecast max supply")
        published_at = _parse_roc_datetime(values[3], "forecast publish time")
        reserve_capacity_mw = max_supply_mw - peak_load_mw
        if peak_load_mw <= 0:
            raise ValueError("Taipower forecast peak load must be positive")
        return TaipowerOperatingReserve(
            date=published_at.date(),
            peak_load_mw=peak_load_mw,
            reserve_capacity_mw=reserve_capacity_mw,
            reserve_rate_percent=reserve_capacity_mw / peak_load_mw * 100,
            is_forecast=True,
            published_at=published_at,
        )

    @classmethod
    def _archive_years_to_fetch(
        cls,
        existing: Iterable[tuple[date, bool]],
        current_year: int,
    ) -> list[int]:
        counts: dict[int, int] = {}
        years_with_forecast: set[int] = set()
        for item_date, is_forecast in existing:
            counts[item_date.year] = counts.get(item_date.year, 0) + 1
            if is_forecast:
                years_with_forecast.add(item_date.year)

        missing: list[int] = []
        for year in range(cls.FIRST_ARCHIVE_YEAR, current_year):
            days_in_year = (date(year + 1, 1, 1) - date(year, 1, 1)).days
            if counts.get(year, 0) < days_in_year or year in years_with_forecast:
                missing.append(year)
        return missing

    async def sync(self, session: AsyncSession) -> int:
        current_year = datetime.now(_TW_TZ).year
        existing_summary = (
            await session.exec(
                select(
                    TaipowerOperatingReserve.date,
                    TaipowerOperatingReserve.is_forecast,
                )
            )
        ).all()
        archive_years = self._archive_years_to_fetch(existing_summary, current_year)
        items = await self.fetch(archive_years, current_year)
        item_dates = [item.date for item in items]
        existing_items = (
            await session.exec(
                select(TaipowerOperatingReserve).where(
                    TaipowerOperatingReserve.date.in_(item_dates)
                )
            )
        ).all()
        existing_by_date = {item.date: item for item in existing_items}

        added = 0
        for item in items:
            existing = existing_by_date.get(item.date)
            if existing is None:
                session.add(item)
                added += 1
            else:
                _update_model(existing, item)
        await session.commit()
        return added


class TaipowerGeneratorCrawler(_TaipowerCrawler):
    """同步台電系統各機組發電量。"""

    INTERVAL = timedelta(minutes=10)
    URL = f"{_TaipowerCrawler.BASE_URL}/data/genary.json"

    async def fetch(self) -> list[TaipowerGenerator]:
        response = await self._get_response(self.URL)
        payload = response.json()
        if not isinstance(payload, dict):
            raise ValueError("Taipower generator response is not a JSON object")
        return self._parse(payload)

    @classmethod
    def _parse(cls, payload: dict[str, Any]) -> list[TaipowerGenerator]:
        published_at = _parse_local_datetime(payload.get(""), "generator publish time")
        rows = payload.get("aaData")
        if not isinstance(rows, list) or not rows:
            raise ValueError("Taipower generator response has no aaData")

        results: list[TaipowerGenerator] = []
        for sequence, row in enumerate(rows):
            if not isinstance(row, list) or len(row) < 7:
                raise ValueError(f"invalid Taipower generator row {sequence}")
            category_html = str(row[0] or "")
            code_match = _CATEGORY_CODE_RE.search(category_html)
            if code_match is None:
                raise ValueError(
                    f"Taipower generator row {sequence} has no category code"
                )
            category = html.unescape(_HTML_TAG_RE.sub("", category_html)).strip()
            unit_name = _optional_text(row[2])
            if not category or unit_name is None:
                raise ValueError(
                    f"Taipower generator row {sequence} has no category or unit"
                )

            installed_capacity, installed_percent = cls._parse_value_with_percent(
                row[3],
                "installed capacity",
            )
            net_generation, generation_percent = cls._parse_value_with_percent(
                row[4],
                "net generation",
            )
            utilization = _optional_text(row[5])
            results.append(
                TaipowerGenerator(
                    published_at=published_at,
                    sequence=sequence,
                    category_code=code_match.group(1),
                    category=category,
                    subcategory=_optional_text(_HTML_TAG_RE.sub("", str(row[1] or ""))),
                    unit_name=unit_name,
                    installed_capacity_mw=installed_capacity,
                    installed_capacity_percent=installed_percent,
                    net_generation_mw=net_generation,
                    net_generation_percent=generation_percent,
                    utilization_percent=(
                        _number(utilization.removesuffix("%"), "utilization")
                        if utilization is not None
                        else None
                    ),
                    status=_optional_text(_HTML_TAG_RE.sub("", str(row[6] or ""))),
                    is_summary="小計" in unit_name,
                )
            )
        return results

    @staticmethod
    def _parse_value_with_percent(
        value: Any,
        field: str,
    ) -> tuple[float | None, float | None]:
        text = _optional_text(value)
        if text is None:
            return None, None
        match = _VALUE_WITH_PERCENT_RE.fullmatch(text)
        if match is None:
            raise ValueError(f"invalid Taipower generator {field}: {value!r}")
        parsed_value = _number(match.group("value"), field)
        percent = match.group("percent")
        return (
            parsed_value,
            _number(percent, f"{field} percent") if percent is not None else None,
        )

    async def sync(self, session: AsyncSession) -> int:
        items = await self.fetch()
        published_at = items[0].published_at
        existing_items = (
            await session.exec(
                select(TaipowerGenerator).where(
                    TaipowerGenerator.published_at == published_at
                )
            )
        ).all()
        existing_by_sequence = {item.sequence: item for item in existing_items}
        added = 0
        for item in items:
            existing = existing_by_sequence.get(item.sequence)
            if existing is None:
                session.add(item)
                added += 1
            else:
                _update_model(existing, item)
        await session.commit()
        return added
