from sunsetRollercoaster.crawler.invoice import InvoiceCrawler
from sunsetRollercoaster.crawler._crawler import Proxy  # 絕對導入
import asyncio

async def main():
    async with InvoiceCrawler(proxy=Proxy.NO) as crawler:
        result = await crawler.query("https://httpbin.org/json")
        print(result)

if __name__ == "__main__":
    asyncio.run(main())