from sunsetRollercoaster.crawler.invoice import InvoiceCrawler,Proxy
import asyncio

async def main():
    async with InvoiceCrawler(proxy=Proxy.NO) as crawler:
        #result = await crawler.query("https://httpbin.org/json")
        #print(result)
        ret = await crawler.client.get("https://www.solarbus.com.tw")
        print(ret.text)

        ret = await crawler.download_file_stream_async("http://http.speed.hinet.net/test_020m.zip","test.zip")

    await crawler.close()

if __name__ == "__main__":
    asyncio.run(main())