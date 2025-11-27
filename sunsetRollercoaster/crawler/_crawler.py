from abc import ABC, abstractmethod
from httpx import AsyncClient,AsyncHTTPTransport
import ua_generator
from enum import Enum
from typing import Any
import truststore

class Proxy(Enum):
    TW = "http://127.0.0.1:1080"
    JP = "http://127.0.0.1:1081"
    NO = None


class Crawler(ABC):
    def __init__(self, proxy: Proxy = Proxy.JP):
        self.proxy = proxy
        self.user_agent = ua_generator.generate()
        headers = {**self.user_agent.headers}

        ctx = truststore.SSLContext()
        self.client = AsyncClient(
            headers=headers,
            proxies={"http://": self.proxy.value, "https://": self.proxy.value},
            timeout=30,
            follow_redirects=True,
            http2=True,
            transport= AsyncHTTPTransport(retries=3, verify=ctx),
        )

    async def close(self) -> None:
        if hasattr(self, 'client') and self.client:
            await self.client.aclose()
    @abstractmethod
    async def query(self, url: str) -> Any:
        pass
