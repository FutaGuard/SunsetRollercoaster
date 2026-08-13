import asyncio
from logging.config import fileConfig

from sqlalchemy import pool
from sqlalchemy.engine import Connection
from sqlalchemy.ext.asyncio import async_engine_from_config
from sqlmodel import SQLModel

from alembic import context
from sunsetRollercoaster import models  # noqa: F401  載入所有 model 註冊到 metadata
from sunsetRollercoaster.config import get_config
from sunsetRollercoaster.database_migration import rename_database_if_needed

LEGACY_DATABASE_NAME = "taiwanreservoir"
DATABASE_NAME = "sunset"
COMMAND_NAME_KEY = "alembic_command_name"
RENAME_AFTER_MIGRATION_KEY = "rename_database_after_migration"

# this is the Alembic Config object, which provides
# access to the values within the .ini file in use.
config = context.config
config.set_main_option("sqlalchemy.url", get_config().database.url)

# Interpret the config file for Python logging.
# This line sets up loggers basically.
if config.config_file_name is not None:
    fileConfig(config.config_file_name)

target_metadata = SQLModel.metadata


def is_upgrade_command() -> bool:
    """Support both the Alembic CLI and the app's programmatic upgrade."""
    if config.attributes.get(COMMAND_NAME_KEY) == "upgrade":
        return True

    command_options = config.cmd_opts
    command = getattr(command_options, "cmd", None)
    return bool(command and command[0].__name__ == "upgrade")


# other values from the config, defined by the needs of env.py,
# can be acquired:
# my_important_option = config.get_main_option("my_important_option")
# ... etc.


def run_migrations_offline() -> None:
    """Run migrations in 'offline' mode.

    This configures the context with just a URL
    and not an Engine, though an Engine is acceptable
    here as well.  By skipping the Engine creation
    we don't even need a DBAPI to be available.

    Calls to context.execute() here emit the given string to the
    script output.

    """
    url = config.get_main_option("sqlalchemy.url")
    context.configure(
        url=url,
        target_metadata=target_metadata,
        literal_binds=True,
        dialect_opts={"paramstyle": "named"},
    )

    with context.begin_transaction():
        context.run_migrations()


def do_run_migrations(connection: Connection) -> None:
    context.configure(connection=connection, target_metadata=target_metadata)

    with context.begin_transaction():
        context.run_migrations()


async def run_async_migrations() -> None:
    """In this scenario we need to create an Engine
    and associate a connection with the context.

    """

    database_url = config.get_main_option("sqlalchemy.url")

    # PostgreSQL refuses to rename the database used by the current session.
    # Perform the one-time rename through the maintenance database before
    # Alembic opens its normal migration connection.
    if is_upgrade_command() and get_config().database.name == DATABASE_NAME:
        renamed = await rename_database_if_needed(
            database_url,
            LEGACY_DATABASE_NAME,
            DATABASE_NAME,
        )
        if renamed:
            config.print_stdout(
                f"Renamed database {LEGACY_DATABASE_NAME!r} to {DATABASE_NAME!r}"
            )

    connectable = async_engine_from_config(
        config.get_section(config.config_ini_section, {}),
        prefix="sqlalchemy.",
        poolclass=pool.NullPool,
    )

    try:
        async with connectable.connect() as connection:
            await connection.run_sync(do_run_migrations)
    finally:
        await connectable.dispose()

    rename_after_migration = config.attributes.pop(
        RENAME_AFTER_MIGRATION_KEY,
        None,
    )
    if rename_after_migration is not None:
        source_name, destination_name = rename_after_migration
        renamed = await rename_database_if_needed(
            database_url,
            source_name,
            destination_name,
        )
        if renamed:
            config.print_stdout(
                f"Renamed database {source_name!r} to {destination_name!r}"
            )


def run_migrations_online() -> None:
    """Run migrations in 'online' mode."""

    asyncio.run(run_async_migrations())


if context.is_offline_mode():
    run_migrations_offline()
else:
    run_migrations_online()
