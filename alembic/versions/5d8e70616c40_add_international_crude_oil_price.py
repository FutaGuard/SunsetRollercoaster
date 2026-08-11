"""add international crude oil prices to nationwide fuel price

Revision ID: 5d8e70616c40
Revises: e7eef413ad8e
Create Date: 2026-08-11 12:00:00.000000

"""

from typing import Sequence, Union

import sqlalchemy as sa
from alembic import op


# revision identifiers, used by Alembic.
revision: str = "5d8e70616c40"
down_revision: Union[str, Sequence[str], None] = "e7eef413ad8e"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    """Upgrade schema."""
    op.add_column(
        "nationwide_fuel_price",
        sa.Column("west_texas", sa.Float(), nullable=True),
    )
    op.add_column(
        "nationwide_fuel_price",
        sa.Column("dubai", sa.Float(), nullable=True),
    )
    op.add_column(
        "nationwide_fuel_price",
        sa.Column("brent", sa.Float(), nullable=True),
    )


def downgrade() -> None:
    """Downgrade schema."""
    op.drop_column("nationwide_fuel_price", "brent")
    op.drop_column("nationwide_fuel_price", "dubai")
    op.drop_column("nationwide_fuel_price", "west_texas")
