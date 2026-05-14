import re
from datetime import datetime, timedelta, timezone
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
        if date is None:
            date = datetime.now(_TW_TZ)
        resp = await self.client.get(self.URL)
        resp.raise_for_status()

        if search == 0 and date.date() == datetime.now(_TW_TZ).date():
            return self._parse(resp.text)

        d = pq(resp.text)
        payload = {
            '__VIEWSTATE': d('input#__VIEWSTATE').attr('value'),
            '__VIEWSTATEGENERATOR': d('input#__VIEWSTATEGENERATOR').attr('value'),
            '__EVENTTARGET': 'ctl00$cphMain$cboSearch',
            '__EVENTARGUMENT': '',
            'ctl00$cphMain$ucDate$cboYear': str(date.year),
            'ctl00$cphMain$ucDate$cboMonth': str(date.month),
            'ctl00$cphMain$ucDate$cboDay': str(date.day),
            'ctl00$cphMain$cboSearch': self.SEARCH[search],
        }
        resp = await self.client.post(self.URL, data=payload)
        resp.raise_for_status()
        return self._parse(resp.text)

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
