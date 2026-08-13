from __future__ import annotations

import logging

from sqlalchemy import text
from sqlalchemy.engine import make_url
from sqlalchemy.ext.asyncio import AsyncConnection, create_async_engine
from sqlalchemy.pool import NullPool

logger = logging.getLogger(__name__)


class DatabaseRenameError(RuntimeError):
    """Raised when a PostgreSQL database cannot be renamed safely."""


async def _database_states(
    connection: AsyncConnection,
    source_name: str,
    destination_name: str,
) -> dict[str, bool]:
    result = await connection.execute(
        text(
            """
            SELECT datname, datallowconn
            FROM pg_database
            WHERE datname IN (:source_name, :destination_name)
            """
        ),
        {
            "source_name": source_name,
            "destination_name": destination_name,
        },
    )
    return {row.datname: row.datallowconn for row in result}


async def _restore_connections(
    connection: AsyncConnection,
    source_name: str,
    destination_name: str,
) -> None:
    """Best-effort recovery if a rename fails after connections are disabled."""
    states = await _database_states(connection, source_name, destination_name)
    active_name = next(
        (name for name in (destination_name, source_name) if name in states),
        None,
    )
    if active_name is None or states[active_name]:
        return

    quoted_name = connection.dialect.identifier_preparer.quote_identifier(active_name)
    await connection.exec_driver_sql(
        f"ALTER DATABASE {quoted_name} ALLOW_CONNECTIONS true"
    )


async def rename_database_if_needed(
    database_url: str,
    source_name: str,
    destination_name: str,
    *,
    maintenance_database: str = "postgres",
) -> bool:
    """Rename a PostgreSQL database through a separate maintenance connection.

    PostgreSQL cannot rename the database used by the current connection.  Alembic
    therefore calls this before opening its migration connection (and after closing
    it for a downgrade).

    Returns ``True`` when a rename was performed and ``False`` when the destination
    already exists.  It refuses to choose between two existing databases so that an
    automated deploy can never overwrite or silently ignore divergent data.
    """
    if not source_name or not destination_name:
        raise ValueError("database names must not be empty")
    if source_name == destination_name:
        return False
    if maintenance_database in {source_name, destination_name}:
        raise ValueError("maintenance database must differ from both database names")

    url = make_url(database_url)
    if url.get_backend_name() != "postgresql":
        raise DatabaseRenameError("database rename migration requires PostgreSQL")

    maintenance_url = url.set(database=maintenance_database)
    engine = create_async_engine(
        maintenance_url,
        isolation_level="AUTOCOMMIT",
        poolclass=NullPool,
    )

    try:
        async with engine.connect() as connection:
            states = await _database_states(
                connection,
                source_name,
                destination_name,
            )

            if source_name in states and destination_name in states:
                raise DatabaseRenameError(
                    f"both databases {source_name!r} and {destination_name!r} exist; "
                    "refusing to choose one automatically"
                )
            if destination_name in states:
                return False
            if source_name not in states:
                raise DatabaseRenameError(
                    f"neither database {source_name!r} nor {destination_name!r} exists"
                )

            source_allowed_connections = states[source_name]
            source_identifier = connection.dialect.identifier_preparer.quote_identifier(
                source_name
            )
            destination_identifier = (
                connection.dialect.identifier_preparer.quote_identifier(
                    destination_name
                )
            )
            connections_disabled = False

            try:
                if source_allowed_connections:
                    await connection.exec_driver_sql(
                        f"ALTER DATABASE {source_identifier} ALLOW_CONNECTIONS false"
                    )
                    connections_disabled = True

                terminated = await connection.execute(
                    text(
                        """
                        SELECT pid, pg_terminate_backend(pid) AS terminated
                        FROM pg_stat_activity
                        WHERE datname = :source_name
                          AND pid <> pg_backend_pid()
                        """
                    ),
                    {"source_name": source_name},
                )
                failed_pids = [str(row.pid) for row in terminated if not row.terminated]
                if failed_pids:
                    raise DatabaseRenameError(
                        "could not terminate database sessions: "
                        + ", ".join(failed_pids)
                    )

                await connection.exec_driver_sql(
                    f"ALTER DATABASE {source_identifier} "
                    f"RENAME TO {destination_identifier}"
                )
                if connections_disabled:
                    await connection.exec_driver_sql(
                        f"ALTER DATABASE {destination_identifier} "
                        "ALLOW_CONNECTIONS true"
                    )
                    connections_disabled = False
            except BaseException:
                if connections_disabled:
                    try:
                        await _restore_connections(
                            connection,
                            source_name,
                            destination_name,
                        )
                    except Exception:
                        logger.exception(
                            "Failed to restore connections after database rename error"
                        )
                raise

            logger.info(
                "Renamed PostgreSQL database %s to %s",
                source_name,
                destination_name,
            )
            return True
    finally:
        await engine.dispose()
