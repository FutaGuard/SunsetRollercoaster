"""rename database to sunset

Revision ID: c3b1f7e9a204
Revises: 5d8e70616c40
Create Date: 2026-08-13 11:00:00.000000

The actual rename is orchestrated by ``alembic/env.py`` through a maintenance
database connection.  PostgreSQL does not allow renaming the database to which
the migration transaction is connected.
"""

from typing import Sequence, Union

import sqlalchemy as sa

from alembic import context, op

revision: str = "c3b1f7e9a204"
down_revision: Union[str, Sequence[str], None] = "5d8e70616c40"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None

LEGACY_DATABASE_NAME = "taiwanreservoir"
DATABASE_NAME = "sunset"
RENAME_AFTER_MIGRATION_KEY = "rename_database_after_migration"


def _current_database() -> str:
    if context.is_offline_mode():
        raise RuntimeError("database rename migration cannot run with --sql")

    connection = op.get_bind()
    if connection.dialect.name != "postgresql":
        raise RuntimeError("database rename migration requires PostgreSQL")
    return connection.scalar(sa.text("SELECT current_database()"))


def upgrade() -> None:
    """Record and verify the database rename performed by the Alembic env."""
    current_database = _current_database()
    if current_database != DATABASE_NAME:
        raise RuntimeError(
            f"expected database {DATABASE_NAME!r}, got {current_database!r}; "
            f"set database.name to {DATABASE_NAME!r} and rerun Alembic"
        )


def downgrade() -> None:
    """Schedule the reverse rename after Alembic closes this connection."""
    current_database = _current_database()
    if current_database != DATABASE_NAME:
        raise RuntimeError(
            f"expected database {DATABASE_NAME!r}, got {current_database!r}"
        )
    context.config.attributes[RENAME_AFTER_MIGRATION_KEY] = (
        DATABASE_NAME,
        LEGACY_DATABASE_NAME,
    )
