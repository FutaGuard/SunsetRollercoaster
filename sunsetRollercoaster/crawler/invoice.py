import re
from datetime import date, timedelta
from typing import Any, Optional

from pyquery import PyQuery as pq
from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from sunsetRollercoaster.models.invoice import Invoice

from ._crawler import Crawler

_PERIOD_RE = re.compile(r'(\d+)年(\d+)-(\d+)月')


class InvoiceCrawler(Crawler):
    INTERVAL = timedelta(days=1)
    URL = 'https://invoice.etax.nat.gov.tw/'
    LAST_URL = 'https://invoice.etax.nat.gov.tw/lastNumber.html'

    def __init__(self, proxy: Crawler.Proxy = Crawler.Proxy.NO):
        super().__init__(proxy)

    async def query(self, url: str) -> Any:
        pass

    async def sync(self, session: AsyncSession) -> int:
        added = 0
        for last in (False, True):
            inv = await self.fetch(last=last)
            if inv is None:
                continue
            existing = (await session.exec(select(Invoice).where(Invoice.date == inv.date))).first()
            if existing is None:
                session.add(inv)
                added += 1
        await session.commit()
        return added

    async def fetch(self, last: bool = False) -> Optional[Invoice]:
        '''
        fetch 取得統一發票中獎號碼
        :param last: True 取上期，預設 False 取當期
        '''
        url = self.LAST_URL if last else self.URL
        resp = await self.client.get(url)
        resp.raise_for_status()
        return self._parse(resp.text)

    @classmethod
    def _parse(cls, html: str) -> Optional[Invoice]:
        d = pq(html)
        table = d('table.etw-table-bgbox.etw-tbig').eq(0)
        if not table.length:
            return None

        special = grand = 0
        first: list[int] = []
        for row in table('tbody tr').items():
            tds = row('td')
            if tds.length < 2:
                continue
            label = tds.eq(0).text().strip()
            value_td = tds.eq(1)
            prize_paragraphs = value_td.find('p.etw-tbiggest')
            if label == '特別獎':
                num = cls._extract_number(prize_paragraphs.eq(0))
                if num is not None:
                    special = num
            elif label == '特獎':
                num = cls._extract_number(prize_paragraphs.eq(0))
                if num is not None:
                    grand = num
            elif label == '頭獎':
                for p in prize_paragraphs.items():
                    num = cls._extract_number(p)
                    if num is not None:
                        first.append(num)

        return Invoice(
            date=cls._parse_period(d) or date.today(),
            special_prize=special,
            grand_prize=grand,
            first_prize=first,
        )

    @staticmethod
    def _parse_period(d) -> Optional[date]:
        title = d('a.etw-on').eq(0).attr('title') or ''
        m = _PERIOD_RE.search(title)
        if not m:
            return None
        roc_year, start_month, _ = (int(x) for x in m.groups())
        return date(roc_year + 1911, start_month, 1)

    @staticmethod
    def _extract_number(p) -> Optional[int]:
        if not p.length:
            return None
        digits = ''.join(c for c in (p.text() or '') if c.isdigit())
        if not digits:
            return None
        try:
            return int(digits)
        except ValueError:
            return None
