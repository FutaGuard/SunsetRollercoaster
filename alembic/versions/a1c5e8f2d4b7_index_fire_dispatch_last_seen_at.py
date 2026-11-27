"""index fire dispatch last_seen_at

Revision ID: a1c5e8f2d4b7
Revises: f3a9c1d7e5b2
Create Date: 2026-11-27 21:00:00.000000

"""

from typing import Sequence, Union

from alembic import op

# revision identifiers, used by Alembic.
revision: str = "a1c5e8f2d4b7"
down_revision: Union[str, Sequence[str], None] = "f3a9c1d7e5b2"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    # `/fire-dispatch/records?since=` polls by last_seen_at.
    op.create_index(
        "ix_fire_dispatch_record_last_seen_at",
        "fire_dispatch_record",
        ["last_seen_at"],
        unique=False,
    )


def downgrade() -> None:
    op.drop_index(
        "ix_fire_dispatch_record_last_seen_at",
        table_name="fire_dispatch_record",
    )
