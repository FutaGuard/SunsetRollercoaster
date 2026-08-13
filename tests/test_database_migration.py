import asyncio
import unittest
from types import SimpleNamespace
from unittest.mock import patch

from sqlalchemy.dialects import postgresql

from sunsetRollercoaster.database_migration import (
    DatabaseRenameError,
    rename_database_if_needed,
)

DATABASE_URL = "postgresql+asyncpg://postgres:test@localhost/sunset"


class FakeConnection:
    def __init__(
        self,
        databases: dict[str, bool],
        sessions: list[tuple[int, bool]] | None = None,
    ) -> None:
        self.databases = databases
        self.sessions = sessions or []
        self.statements: list[str] = []
        self.dialect = postgresql.dialect()

    async def execute(self, statement, parameters):
        sql = str(statement)
        if "FROM pg_database" in sql:
            return [
                SimpleNamespace(datname=name, datallowconn=allows_connections)
                for name, allows_connections in self.databases.items()
                if name
                in {
                    parameters["source_name"],
                    parameters["destination_name"],
                }
            ]
        if "pg_terminate_backend" in sql:
            return [
                SimpleNamespace(pid=pid, terminated=terminated)
                for pid, terminated in self.sessions
            ]
        raise AssertionError(f"unexpected SQL: {sql}")

    async def exec_driver_sql(self, statement: str) -> None:
        self.statements.append(statement)
        if statement == 'ALTER DATABASE "taiwanreservoir" ALLOW_CONNECTIONS false':
            self.databases["taiwanreservoir"] = False
        elif statement == 'ALTER DATABASE "taiwanreservoir" RENAME TO "sunset"':
            self.databases["sunset"] = self.databases.pop("taiwanreservoir")
        elif statement == 'ALTER DATABASE "sunset" ALLOW_CONNECTIONS true':
            self.databases["sunset"] = True
        elif statement == 'ALTER DATABASE "taiwanreservoir" ALLOW_CONNECTIONS true':
            self.databases["taiwanreservoir"] = True
        else:
            raise AssertionError(f"unexpected driver SQL: {statement}")


class FakeConnectionContext:
    def __init__(self, connection: FakeConnection) -> None:
        self.connection = connection

    async def __aenter__(self) -> FakeConnection:
        return self.connection

    async def __aexit__(self, exc_type, exc_value, traceback) -> None:
        return None


class FakeEngine:
    def __init__(self, connection: FakeConnection) -> None:
        self.connection = connection
        self.disposed = False

    def connect(self) -> FakeConnectionContext:
        return FakeConnectionContext(self.connection)

    async def dispose(self) -> None:
        self.disposed = True


class DatabaseMigrationTest(unittest.TestCase):
    def run_rename(self, connection: FakeConnection) -> tuple[bool, FakeEngine]:
        engine = FakeEngine(connection)
        with patch(
            "sunsetRollercoaster.database_migration.create_async_engine",
            return_value=engine,
        ) as create_engine:
            renamed = asyncio.run(
                rename_database_if_needed(
                    DATABASE_URL,
                    "taiwanreservoir",
                    "sunset",
                )
            )

        maintenance_url = create_engine.call_args.args[0]
        self.assertEqual(maintenance_url.database, "postgres")
        self.assertEqual(
            create_engine.call_args.kwargs["isolation_level"], "AUTOCOMMIT"
        )
        return renamed, engine

    def test_renames_database_and_preserves_connection_state(self):
        connection = FakeConnection(
            {"taiwanreservoir": True},
            sessions=[(101, True)],
        )

        renamed, engine = self.run_rename(connection)

        self.assertTrue(renamed)
        self.assertEqual(connection.databases, {"sunset": True})
        self.assertEqual(
            connection.statements,
            [
                'ALTER DATABASE "taiwanreservoir" ALLOW_CONNECTIONS false',
                'ALTER DATABASE "taiwanreservoir" RENAME TO "sunset"',
                'ALTER DATABASE "sunset" ALLOW_CONNECTIONS true',
            ],
        )
        self.assertTrue(engine.disposed)

    def test_is_idempotent_when_destination_exists(self):
        connection = FakeConnection({"sunset": True})

        renamed, engine = self.run_rename(connection)

        self.assertFalse(renamed)
        self.assertEqual(connection.statements, [])
        self.assertTrue(engine.disposed)

    def test_refuses_to_choose_when_both_databases_exist(self):
        connection = FakeConnection({"taiwanreservoir": True, "sunset": True})
        engine = FakeEngine(connection)

        with (
            patch(
                "sunsetRollercoaster.database_migration.create_async_engine",
                return_value=engine,
            ),
            self.assertRaisesRegex(DatabaseRenameError, "both databases"),
        ):
            asyncio.run(
                rename_database_if_needed(
                    DATABASE_URL,
                    "taiwanreservoir",
                    "sunset",
                )
            )

        self.assertEqual(connection.statements, [])
        self.assertTrue(engine.disposed)

    def test_restores_connections_when_a_session_cannot_be_terminated(self):
        connection = FakeConnection(
            {"taiwanreservoir": True},
            sessions=[(202, False)],
        )
        engine = FakeEngine(connection)

        with (
            patch(
                "sunsetRollercoaster.database_migration.create_async_engine",
                return_value=engine,
            ),
            self.assertRaisesRegex(DatabaseRenameError, "202"),
        ):
            asyncio.run(
                rename_database_if_needed(
                    DATABASE_URL,
                    "taiwanreservoir",
                    "sunset",
                )
            )

        self.assertEqual(connection.databases, {"taiwanreservoir": True})
        self.assertEqual(
            connection.statements,
            [
                'ALTER DATABASE "taiwanreservoir" ALLOW_CONNECTIONS false',
                'ALTER DATABASE "taiwanreservoir" ALLOW_CONNECTIONS true',
            ],
        )
        self.assertTrue(engine.disposed)


if __name__ == "__main__":
    unittest.main()
