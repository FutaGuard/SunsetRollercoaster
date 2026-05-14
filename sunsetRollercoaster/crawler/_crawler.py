from abc import ABC, abstractmethod
from datetime import timedelta
from enum import Enum
from typing import Any, ClassVar

import httpx
import truststore
import ua_generator
from httpx import AsyncClient, AsyncHTTPTransport
from sqlmodel.ext.asyncio.session import AsyncSession


class Crawler(ABC):
    INTERVAL: ClassVar[timedelta]

    class Proxy(Enum):
        TW = "http://127.0.0.1:1080"
        JP = "http://127.0.0.1:1081"
        NO = None

    def __init__(self, proxy: Proxy = Proxy.NO):
        self.proxy = proxy
        self.user_agent = ua_generator.generate()
        headers = {
            **self.user_agent.headers.get(),
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
            "Accept-Language": "zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.7",
            "Accept-Encoding": "gzip, deflate, br",
            "Connection": "keep-alive",
            "Upgrade-Insecure-Requests": "1",
            "Sec-Fetch-Dest": "document",
            "Sec-Fetch-Mode": "navigate",
            "Sec-Fetch-Site": "none",
            "Sec-Fetch-User": "?1",
            "Cache-Control": "max-age=0",
        }

        ctx = truststore.SSLContext()
        self.client = AsyncClient(
            headers=headers,
            follow_redirects=True,
            http2=True,
            transport=AsyncHTTPTransport(retries=3, verify=ctx, proxy=self.proxy.value),
            timeout=httpx.Timeout(
                connect=10.0,  # 建立連線
                read=200.0,  # 等回應（這個調大）
                write=10.0,  # 送資料
                pool=10.0,  # 等連線池
            ),
        )
    async def close(self) -> None:
        if hasattr(self, 'client') and self.client:
            await self.client.aclose()
    async def __aenter__(self):
        return self
    async def download_file_stream_async(self,url: str, save_path: str, chunk_size: int = 8192):
        result ={}
        try:
            async with httpx.AsyncClient() as client:
                async with client.stream("GET", url) as response:
                    response.raise_for_status()
                    with open(save_path, "wb") as f:
                        async for chunk in response.aiter_bytes(chunk_size=chunk_size):
                            if chunk:
                                f.write(chunk)
            result = {
                'success':True,
                'reason':'檔案下載完成'
            }
        except Exception as e:
            result={
                'success':False,
                'status_code':e.response.status_code,
                'reason': '檔案下載失敗',
                'text':e.response.text
            }
        return result
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()

    @abstractmethod
    async def query(self, url: str) -> Any:
        pass

    @abstractmethod
    async def sync(self, session: AsyncSession) -> int:
        '''抓取資料、與 DB 比對，寫入新資料並回傳新增筆數'''
        pass
