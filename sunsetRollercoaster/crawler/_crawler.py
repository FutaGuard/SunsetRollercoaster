from abc import ABC, abstractmethod
from httpx import AsyncClient
import ua_generator
from enum import Enum
from typing import Any

class Proxy(Enum):
    TW = "http://127.0.0.1:1080"
    JP = "http://127.0.0.1:1081"
    NO = None


class Crawler(ABC):
    def __init__(self, proxy: Proxy = Proxy.JP):
        self.proxy = proxy
        self.user_agent = ua_generator.generate()
        headers = {**self.user_agent.headers}
        self.client = AsyncClient(headers=headers, proxies={"http://": self.proxy.value, "https://": self.proxy.value})
    
    @abstractmethod
    def query(self, url: str) -> Any:
        pass
