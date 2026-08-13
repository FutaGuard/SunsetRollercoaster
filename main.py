import asyncio
import sys
from datetime import datetime
from pathlib import Path
from typing import Type

from alembic.config import Config as AlembicConfig
from apscheduler.schedulers.asyncio import AsyncIOScheduler
from apscheduler.triggers.interval import IntervalTrigger
from loguru import logger
from sqlalchemy.ext.asyncio import async_sessionmaker, create_async_engine
from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession

from alembic import command
from sunsetRollercoaster import models  # noqa: F401  載入 metadata
from sunsetRollercoaster.config import get_config
from sunsetRollercoaster.crawler._crawler import Crawler
from sunsetRollercoaster.crawler.fuel_price import NationwideFuelPriceCrawler
from sunsetRollercoaster.crawler.invoice import InvoiceCrawler
from sunsetRollercoaster.crawler.reservoir import ReservoirCrawler
from sunsetRollercoaster.crawler.taipower import (
    TaipowerAreaCrawler,
    TaipowerFuelMixCrawler,
    TaipowerGeneratorCrawler,
    TaipowerOperatingReserveCrawler,
    TaipowerPowerSnapshotCrawler,
)

CRAWLERS: list[Type[Crawler]] = [
    InvoiceCrawler,
    ReservoirCrawler,
    NationwideFuelPriceCrawler,
    TaipowerPowerSnapshotCrawler,
    TaipowerFuelMixCrawler,
    TaipowerAreaCrawler,
    TaipowerOperatingReserveCrawler,
    TaipowerGeneratorCrawler,
]

PROJECT_ROOT = Path(__file__).resolve().parent
ALEMBIC_COMMAND_NAME_KEY = "alembic_command_name"


def migrate_database() -> None:
    """Apply all pending Alembic migrations before starting scheduled jobs."""
    alembic_config = AlembicConfig(PROJECT_ROOT / "alembic.ini")
    alembic_config.attributes[ALEMBIC_COMMAND_NAME_KEY] = "upgrade"
    command.upgrade(alembic_config, "head")


async def init_db(engine, reset: bool = False):
    async with engine.begin() as conn:
        if reset:
            await conn.run_sync(SQLModel.metadata.drop_all)
        await conn.run_sync(SQLModel.metadata.create_all)


async def run_crawler(
    cls: Type[Crawler], session_factory: async_sessionmaker[AsyncSession]
):
    name = cls.__name__
    try:
        async with cls() as crawler, session_factory() as session:
            added = await crawler.sync(session)
        logger.info(f"{name} 寫入 {added} 筆")
    except Exception:
        logger.exception(f"{name} 執行失敗")


async def main():
    engine = create_async_engine(get_config().database.url)
    await init_db(engine, reset="--reset" in sys.argv)
    session_factory = async_sessionmaker(
        engine, class_=AsyncSession, expire_on_commit=False
    )

    if "--once" in sys.argv:
        for cls in CRAWLERS:
            await run_crawler(cls, session_factory)
        await engine.dispose()
        return

    scheduler = AsyncIOScheduler()
    for cls in CRAWLERS:
        scheduler.add_job(
            run_crawler,
            trigger=IntervalTrigger(seconds=cls.INTERVAL.total_seconds()),
            args=[cls, session_factory],
            id=cls.__name__,
            next_run_time=datetime.now(),
            max_instances=1,
            coalesce=True,
        )
        logger.info(f"排程 {cls.__name__} 每 {cls.INTERVAL} 執行一次")

    scheduler.start()
    logger.info("scheduler 啟動，Ctrl+C 結束")
    try:
        await asyncio.Event().wait()
    finally:
        scheduler.shutdown()
        await engine.dispose()


if __name__ == "__main__":
    try:
        migrate_database()
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("結束")
