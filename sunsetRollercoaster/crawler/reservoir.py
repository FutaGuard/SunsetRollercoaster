import asyncio
import re
from datetime import date, datetime, timedelta, timezone
from typing import Any, Optional

from pyquery import PyQuery as pq
from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from sunsetRollercoaster.models.reservoir import Reservoir

from ._crawler import Crawler

_TW_TZ = timezone(timedelta(hours=8))
_DATE_RE = re.compile(r'(\d{4})-(\d{1,2})-(\d{1,2})\((\d+)時\)')
_BR_RE = re.compile(r'<br\s*/?>', re.IGNORECASE)
_TAG_RE = re.compile(r'<[^>]+>')


class ReservoirCrawler(Crawler):
    INTERVAL = timedelta(hours=1)
    URL = 'https://fhy.wra.gov.tw/ReservoirPage_2011/StorageCapacity.aspx'
    STATION_URL = 'https://fhy.wra.gov.tw/Api/v2/Reservoir/Station'
    REALTIME_URL = 'https://fhy.wra.gov.tw/Api/v2/Reservoir/Info/RealTime'
    DAILY_URL = 'https://fhy.wra.gov.tw/OpenApiv3/v2/Reservoir/Daily'
    API_KEY = 'd6dd3cd4-493f-43a3-92b1-8b2db217da96'
    FOCUS_STATION_NOS = {
        '10201',  # 石門水庫
        '10204',  # 新山水庫
        '10205',  # 翡翠水庫
        '10405',  # 寶山第二水庫
        '10501',  # 永和山水庫
        '10601',  # 明德水庫
        '20101',  # 鯉魚潭水庫
        '20201',  # 德基水庫
        '20202',  # 石岡壩
        '20501',  # 霧社水庫
        '20502',  # 日月潭水庫
        '20503',  # 集集攔河堰
        '20509',  # 湖山水庫
        '30301',  # 仁義潭水庫
        '30401',  # 白河水庫
        '30501',  # 烏山頭水庫
        '30502',  # 曾文水庫
        '30503',  # 南化水庫
        '30802',  # 阿公店水庫
        '30901',  # 高屏溪攔河堰
        '31201',  # 牡丹水庫
    }
    SEARCH = {0: '防汛重點水庫', 1: '所有水庫', 2: '水庫及攔河堰'}

    def __init__(self, proxy: Crawler.Proxy = Crawler.Proxy.NO):
        super().__init__(proxy)

    async def query(self, url: str) -> Any:
        pass

    async def sync(self, session: AsyncSession) -> int:
        items = await self.fetch()
        added = 0
        for r in items:
            stmt = select(Reservoir).where(
                Reservoir.name == r.name,
                Reservoir.recordTime == r.recordTime,
            )
            existing = (await session.exec(stmt)).first()
            if existing is None:
                session.add(r)
                added += 1
        await session.commit()
        return added

    async def fetch(self, date: Optional[datetime] = None, search: int = 0) -> list[Reservoir]:
        '''
        fetch 取得指定日期的水庫資料
        :param date: 查詢日期，預設為當下台灣時區時間
        :param search: 0=防汛重點水庫, 1=所有水庫, 2=水庫及攔河堰
        '''
        if search not in self.SEARCH:
            raise ValueError(f'unknown reservoir search option: {search}')

        stations, realtime_items, daily_items = await asyncio.gather(
            self._fetch_api_data(self.STATION_URL),
            self._fetch_api_data(self.REALTIME_URL),
            self._fetch_api_data(self.DAILY_URL),
        )

        station_by_no = {
            str(item.get('StationNo')): item
            for item in stations
            if item.get('StationNo') is not None
        }
        daily_by_no = {
            str(item.get('StationNo')): item
            for item in daily_items
            if item.get('StationNo') is not None
        }
        station_nos = self._station_numbers_for_search(search, station_by_no, realtime_items)
        target_date = self._target_date(date)

        results: list[Reservoir] = []
        for item in realtime_items:
            station_no = str(item.get('StationNo'))
            if station_no not in station_nos:
                continue

            record_time = self._to_api_datetime(item.get('Time'))
            if target_date is not None:
                if record_time is None or record_time.astimezone(_TW_TZ).date() != target_date:
                    continue

            station = station_by_no.get(station_no, {})
            daily = daily_by_no.get(station_no, {})
            capavailable = self._json_float(item.get('EffectiveCapacity'))
            if capavailable is None:
                capavailable = self._json_float(station.get('EffectiveCapacity'))
            currcapper = self._json_float(item.get('PercentageOfStorage'))
            if station_no == '30901':
                # The official site suppresses this percentage for 高屏溪攔河堰.
                currcapper = None

            results.append(Reservoir(
                name=station.get('StationName'),
                capavailable=capavailable,
                statisticTimeS=None,
                statisticTimeE=None,
                rainFall=self._json_float(item.get('AccumulatedRainfall')),
                inFlow=self._json_float(item.get('Inflow')),
                outFlow=self._json_float(item.get('Outflow')),
                waterlevediff=self._json_float(daily.get('Difference')),
                recordTime=record_time,
                caplevel=self._json_float(item.get('WaterHeight')),
                currcap=self._json_float(item.get('EffectiveStorage')),
                currcapper=currcapper,
            ))
        return results

    async def _fetch_api_data(self, url: str) -> list[dict[str, Any]]:
        resp = await self.client.get(url, headers={
            'Accept': 'application/json',
            'apikey': self.API_KEY,
        })
        resp.raise_for_status()
        payload = resp.json()
        data = payload.get('Data') if isinstance(payload, dict) else None
        return data if isinstance(data, list) else []

    @classmethod
    def _station_numbers_for_search(
        cls,
        search: int,
        station_by_no: dict[str, dict[str, Any]],
        realtime_items: list[dict[str, Any]],
    ) -> set[str]:
        if search == 0:
            return cls.FOCUS_STATION_NOS
        if search == 1:
            return {
                station_no
                for station_no, station in station_by_no.items()
                if station.get('HydraulicConstruction') == 1
            }
        return {
            str(item.get('StationNo'))
            for item in realtime_items
            if item.get('StationNo') is not None
        }

    @staticmethod
    def _target_date(date: Optional[datetime]) -> Optional[date]:
        if date is None:
            return None
        if date.tzinfo is None:
            return date.date()
        return date.astimezone(_TW_TZ).date()

    @staticmethod
    def _to_api_datetime(value: Any) -> Optional[datetime]:
        if not value:
            return None
        s = str(value).strip()
        if s.endswith('Z'):
            s = f'{s[:-1]}+00:00'
        try:
            dt = datetime.fromisoformat(s)
        except ValueError:
            return None
        if dt.tzinfo is None:
            return dt.replace(tzinfo=_TW_TZ)
        return dt.astimezone(_TW_TZ)

    @staticmethod
    def _json_float(value: Any) -> Optional[float]:
        if value is None:
            return None
        try:
            return float(value)
        except (TypeError, ValueError):
            return None

    @classmethod
    def _parse(cls, html: str) -> list[Reservoir]:
        d = pq(html)
        table = d('table#ctl00_cphMain_gvList')
        if not table.length:
            return []
        results: list[Reservoir] = []
        for row in list(table('tr').items())[2:-1]:
            tds = row('td')
            if tds.length < 11:
                continue
            time_lines = cls._split_br(tds.eq(2))
            results.append(Reservoir(
                name=tds.eq(0).text().strip() or None,
                capavailable=cls._to_float(tds.eq(1).text()),
                statisticTimeS=cls._to_datetime(time_lines[0]) if len(time_lines) > 0 else None,
                statisticTimeE=cls._to_datetime(time_lines[1]) if len(time_lines) > 1 else None,
                rainFall=cls._to_float(tds.eq(3).text()),
                inFlow=cls._to_float(tds.eq(4).text()),
                outFlow=cls._to_float(tds.eq(5).text()),
                waterlevediff=cls._to_float(tds.eq(6).text()),
                recordTime=cls._to_datetime(tds.eq(7).text()),
                caplevel=cls._to_float(tds.eq(8).text()),
                currcap=cls._to_float(tds.eq(9).text()),
                currcapper=cls._to_float(tds.eq(10).text()),
            ))
        return results

    @staticmethod
    def _split_br(node) -> list[str]:
        # pyquery / lxml 的 .text() 會把 <br/> 折疊掉，所以從 inner HTML 切
        inner = node.html() or ''
        parts = (_TAG_RE.sub('', p).strip() for p in _BR_RE.split(inner))
        return [p for p in parts if p]

    @staticmethod
    def _to_float(s: str) -> Optional[float]:
        s = s.replace('%', '').replace(',', '').strip()
        if not s or '--' in s:
            return None
        try:
            return float(s)
        except ValueError:
            return None

    @staticmethod
    def _to_datetime(s: str) -> Optional[datetime]:
        m = _DATE_RE.search(s)
        if not m:
            return None
        y, mo, d, h = (int(x) for x in m.groups())
        return datetime(y, mo, d, h, tzinfo=_TW_TZ)
