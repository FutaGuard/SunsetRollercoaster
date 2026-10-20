"""add finance exchange rate

Revision ID: b8d4f2a6c913
Revises: a6f2d9c4e781
Create Date: 2026-10-20 11:30:00.000000

"""

from typing import Sequence, Union

import sqlalchemy as sa

from alembic import op

# revision identifiers, used by Alembic.
revision: str = "b8d4f2a6c913"
down_revision: Union[str, Sequence[str], None] = "a6f2d9c4e781"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "finance_exchange_rate",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("base", sa.String(length=3), nullable=False),
        sa.Column("quote", sa.String(length=3), nullable=False),
        sa.Column("rate", sa.Float(), nullable=False),
        sa.Column("observed_at", sa.DateTime(timezone=True), nullable=False),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint(
            "base",
            "quote",
            "observed_at",
            name="uq_finance_exchange_rate_pair_observed_at",
        ),
    )
    op.create_index(
        "ix_finance_exchange_rate_observed_at",
        "finance_exchange_rate",
        ["observed_at"],
        unique=False,
    )


def downgrade() -> None:
    op.drop_index(
        "ix_finance_exchange_rate_observed_at",
        table_name="finance_exchange_rate",
    )
    op.drop_table("finance_exchange_rate")
