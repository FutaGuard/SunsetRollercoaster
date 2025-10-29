from typing import Any
from ._crawler import Crawler, Proxy


class InvoiceCrawler(Crawler):
    def __init__(self, proxy: Proxy = Proxy.JP):
        super().__init__(proxy)
    
    async def query(self, url: str) -> Any:
        """實現抽象方法 query"""
        # TODO: 實現發票爬取邏輯
        response = await self.client.get(url)
        return response.json()
    