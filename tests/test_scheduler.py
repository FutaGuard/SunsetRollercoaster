import asyncio
import unittest
from contextlib import suppress
from datetime import datetime, timedelta
from types import SimpleNamespace
from unittest.mock import AsyncMock, Mock, patch

import main as application


class SchedulerStartupTest(unittest.IsolatedAsyncioTestCase):
    async def test_runs_startup_sync_when_initialization_takes_over_one_second(self):
        class TestCrawler:
            INTERVAL = timedelta(days=1)

        completed = asyncio.Event()
        calls = []
        factory = object()
        engine = SimpleNamespace(dispose=AsyncMock())

        async def record_run(crawler, session_factory):
            calls.append((crawler, session_factory))
            completed.set()

        with patch.multiple(
            application,
            CRAWLERS=[TestCrawler],
            datetime=Mock(now=Mock(return_value=datetime.now() - timedelta(seconds=2))),
            get_config=Mock(return_value=SimpleNamespace(
                database=SimpleNamespace(url="postgresql+asyncpg://unused")
            )),
            create_async_engine=Mock(return_value=engine),
            async_sessionmaker=Mock(return_value=factory),
            init_db=AsyncMock(),
            run_crawler=record_run,
        ):
            task = asyncio.create_task(application.main())
            try:
                await asyncio.wait_for(completed.wait(), timeout=2)
            finally:
                task.cancel()
                with suppress(asyncio.CancelledError):
                    await task

        self.assertEqual(calls, [(TestCrawler, factory)])
        engine.dispose.assert_awaited_once()


if __name__ == "__main__":
    unittest.main()
