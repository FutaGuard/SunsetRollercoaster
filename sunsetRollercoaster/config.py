from functools import lru_cache
from pathlib import Path
from urllib.parse import quote_plus

import yaml
from pydantic import BaseModel


class DatabaseConfig(BaseModel):
    host: str
    port: int
    user: str
    password: str
    name: str
    driver: str = "postgresql+asyncpg"

    @property
    def url(self) -> str:
        return (
            f"{self.driver}://{quote_plus(self.user)}:{quote_plus(self.password)}"
            f"@{self.host}:{self.port}/{self.name}"
        )


class FinanceConfig(BaseModel):
    # Apps Script Web App 網址只放在 env_config.yml，不進 repo。
    exchange_rate_url: str


class Config(BaseModel):
    database: DatabaseConfig
    finance: FinanceConfig | None = None


_CONFIG_PATH = Path(__file__).resolve().parent.parent / "env_config.yml"


@lru_cache
def get_config() -> Config:
    with open(_CONFIG_PATH) as f:
        data = yaml.safe_load(f)
    return Config.model_validate(data)
