from datetime import datetime, timedelta, timezone
from typing import Any

from loguru import logger
from sqlalchemy import literal_column
from sqlalchemy.dialects.postgresql import insert
from sqlmodel.ext.asyncio.session import AsyncSession

from sunsetRollercoaster.config import get_config
from sunsetRollercoaster.models.fire_dispatch import FireDispatchRecord

from ._crawler import Crawler

_STATE_COLUMNS = (
    "county",
    "received_at",
    "category",
    "location",
    "unit",
    "status",
    "case_id",
)


class FireDispatchRecordCrawler(Crawler):
    """把 sunset-api `/fire-dispatch/latest` 的即時案件累積進資料庫。

    各縣市頁面的解析、代理與快取都在 sunset-api，這裡不重複實作；
    sunset-api 位址設定於 env_config.yml 的 `sunset_api.url`。
    即時列表只保留 6 小時內的案件，需持續同步才留得下每次狀態變化。
    sunset-api 每縣市快取 60 秒；以一半的間隔讀取，才不會因週期對齊而連續拿到同一份快取。
    """

    INTERVAL = timedelta(seconds=30)

    def __init__(self, proxy: Crawler.Proxy = Crawler.Proxy.NO):
        super().__init__(proxy)

    async def query(self, url: str) -> list[Any]:
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
        if not isinstance(payload, list):
            raise ValueError("fire dispatch response is not a JSON array")
        return payload

    async def fetch(self) -> list[FireDispatchRecord]:
        sunset_api = get_config().sunset_api
        if sunset_api is None:
            raise ValueError("env_config.yml 未設定 sunset_api.url")
        url = f"{sunset_api.url.rstrip('/')}/fire-dispatch/latest"
        return self._parse(await self.query(url))

    async def sync(self, session: AsyncSession) -> int:
        records = await self.fetch()
        if not records:
            return 0
        rows = [record.model_dump(exclude={"id"}) for record in records]
        statement = insert(FireDispatchRecord).values(rows)
        statement = statement.on_conflict_do_update(
            constraint="uq_fire_dispatch_record_state",
            set_={
                "last_seen_at": statement.excluded.last_seen_at,
                "latitude": statement.excluded.latitude,
                "longitude": statement.excluded.longitude,
            },
            # PostgreSQL 的 xmax 在新插入的列為 0，被更新的既有列則不是。
        ).returning(literal_column("xmax = 0"))
        result = await session.exec(statement)
        added = sum(1 for (inserted,) in result.all() if inserted)
        await session.commit()
        return added

    @classmethod
    def _parse(cls, payload: list[Any]) -> list[FireDispatchRecord]:
        records: dict[tuple, FireDispatchRecord] = {}
        for county in payload:
            if not isinstance(county, dict):
                raise ValueError("fire dispatch county is not a JSON object")
            code = county.get("code")
            if not isinstance(code, str) or not code:
                raise ValueError(f"invalid fire dispatch county code: {code!r}")
            if county.get("error"):
                logger.warning(f"消防案件 {code} 來源失敗：{county['error']}")
                continue
            seen_at = cls._timestamp(county.get("fetched_at"), "fetched_at")
            for case in county.get("cases") or []:
                record = FireDispatchRecord(
                    county=code,
                    received_at=(
                        None
                        if case.get("received_at") is None
                        else cls._timestamp(case["received_at"], "received_at")
                    ),
                    category=case.get("category") or "",
                    location=case.get("location") or "",
                    unit=case.get("unit"),
                    status=case.get("status"),
                    case_id=case.get("case_id"),
                    latitude=case.get("latitude"),
                    longitude=case.get("longitude"),
                    first_seen_at=seen_at,
                    last_seen_at=seen_at,
                )
                # 同一列表偶有完全相同的列（例如同時派遣兩車）；同一批次重複會讓 upsert 失敗。
                records[tuple(getattr(record, name) for name in _STATE_COLUMNS)] = record
        return list(records.values())

    @staticmethod
    def _timestamp(value: Any, field: str) -> datetime:
        if not isinstance(value, str):
            raise ValueError(f"invalid fire dispatch {field}: {value!r}")
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            raise ValueError(f"invalid fire dispatch {field}: {value!r}")
        return parsed.astimezone(timezone.utc)
