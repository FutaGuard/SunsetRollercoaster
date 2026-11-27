"""add fire dispatch record

Revision ID: f3a9c1d7e5b2
Revises: b8d4f2a6c913
Create Date: 2026-11-27 10:00:00.000000

"""

from typing import Sequence, Union

import sqlalchemy as sa

from alembic import op

# revision identifiers, used by Alembic.
revision: str = "f3a9c1d7e5b2"
down_revision: Union[str, Sequence[str], None] = "b8d4f2a6c913"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "fire_dispatch_record",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("county", sa.String(length=32), nullable=False),
        sa.Column("received_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("category", sa.Text(), nullable=False),
        sa.Column("location", sa.Text(), nullable=False),
        sa.Column("unit", sa.Text(), nullable=True),
        sa.Column("status", sa.Text(), nullable=True),
        sa.Column("case_id", sa.Text(), nullable=True),
        sa.Column("latitude", sa.Float(), nullable=True),
        sa.Column("longitude", sa.Float(), nullable=True),
        sa.Column("first_seen_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("last_seen_at", sa.DateTime(timezone=True), nullable=False),
        sa.PrimaryKeyConstraint("id"),
        # NULLS NOT DISTINCT requires PostgreSQL 15+.
        sa.UniqueConstraint(
            "county",
            "received_at",
            "category",
            "location",
            "unit",
            "status",
            "case_id",
            name="uq_fire_dispatch_record_state",
            postgresql_nulls_not_distinct=True,
        ),
    )
    op.create_index(
        "ix_fire_dispatch_record_first_seen_at",
        "fire_dispatch_record",
        ["first_seen_at"],
        unique=False,
    )
    op.create_index(
        "ix_fire_dispatch_record_county_first_seen_at",
        "fire_dispatch_record",
        ["county", "first_seen_at"],
        unique=False,
    )


def downgrade() -> None:
    op.drop_index(
        "ix_fire_dispatch_record_county_first_seen_at",
        table_name="fire_dispatch_record",
    )
    op.drop_index(
        "ix_fire_dispatch_record_first_seen_at",
        table_name="fire_dispatch_record",
    )
    op.drop_table("fire_dispatch_record")
