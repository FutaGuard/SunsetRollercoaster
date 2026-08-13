"""add Taipower data

Revision ID: a6f2d9c4e781
Revises: c3b1f7e9a204
Create Date: 2026-08-13 11:30:00.000000

"""

from typing import Sequence, Union

import sqlalchemy as sa

from alembic import op

# revision identifiers, used by Alembic.
revision: str = "a6f2d9c4e781"
down_revision: Union[str, Sequence[str], None] = "c3b1f7e9a204"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "taipower_power_snapshot",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("published_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("current_load_mw", sa.Float(), nullable=True),
        sa.Column("current_utilization_percent", sa.Float(), nullable=True),
        sa.Column("forecast_max_supply_mw", sa.Float(), nullable=True),
        sa.Column("forecast_peak_demand_mw", sa.Float(), nullable=True),
        sa.Column("forecast_peak_reserve_mw", sa.Float(), nullable=True),
        sa.Column("forecast_peak_reserve_rate_percent", sa.Float(), nullable=True),
        sa.Column(
            "forecast_peak_reserve_indicator", sa.String(length=16), nullable=True
        ),
        sa.Column("forecast_peak_hour_range", sa.String(length=32), nullable=True),
        sa.Column("yesterday_date", sa.Date(), nullable=True),
        sa.Column("yesterday_max_supply_mw", sa.Float(), nullable=True),
        sa.Column("yesterday_peak_demand_mw", sa.Float(), nullable=True),
        sa.Column("yesterday_peak_reserve_mw", sa.Float(), nullable=True),
        sa.Column(
            "yesterday_peak_reserve_rate_percent",
            sa.Float(),
            nullable=True,
        ),
        sa.Column(
            "yesterday_peak_reserve_indicator",
            sa.String(length=16),
            nullable=True,
        ),
        sa.Column("realtime_max_supply_mw", sa.Float(), nullable=True),
        sa.Column("realtime_peak_at", sa.DateTime(timezone=True), nullable=True),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint(
            "published_at",
            name="uq_taipower_power_snapshot_published_at",
        ),
    )
    op.create_index(
        "ix_taipower_power_snapshot_published_at",
        "taipower_power_snapshot",
        ["published_at"],
        unique=False,
    )

    op.create_table(
        "taipower_fuel_mix",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("observed_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("lng_mw", sa.Float(), nullable=False),
        sa.Column("ipp_lng_mw", sa.Float(), nullable=False),
        sa.Column("coal_mw", sa.Float(), nullable=False),
        sa.Column("ipp_coal_mw", sa.Float(), nullable=False),
        sa.Column("cogeneration_mw", sa.Float(), nullable=False),
        sa.Column("fuel_oil_mw", sa.Float(), nullable=False),
        sa.Column("solar_mw", sa.Float(), nullable=False),
        sa.Column("wind_mw", sa.Float(), nullable=False),
        sa.Column("hydro_mw", sa.Float(), nullable=False),
        sa.Column("energy_storage_mw", sa.Float(), nullable=False),
        sa.Column("other_renewable_mw", sa.Float(), nullable=False),
        sa.Column("energy_storage_load_mw", sa.Float(), nullable=False),
        sa.Column("total_mw", sa.Float(), nullable=False),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint(
            "observed_at",
            name="uq_taipower_fuel_mix_observed_at",
        ),
    )
    op.create_index(
        "ix_taipower_fuel_mix_observed_at",
        "taipower_fuel_mix",
        ["observed_at"],
        unique=False,
    )

    op.create_table(
        "taipower_area_load",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("observed_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("north_load_mw", sa.Float(), nullable=False),
        sa.Column("central_load_mw", sa.Float(), nullable=False),
        sa.Column("south_load_mw", sa.Float(), nullable=False),
        sa.Column("east_load_mw", sa.Float(), nullable=False),
        sa.Column("total_load_mw", sa.Float(), nullable=False),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint(
            "observed_at",
            name="uq_taipower_area_load_observed_at",
        ),
    )
    op.create_index(
        "ix_taipower_area_load_observed_at",
        "taipower_area_load",
        ["observed_at"],
        unique=False,
    )

    op.create_table(
        "taipower_area_snapshot",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("observed_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("north_generation_mw", sa.Float(), nullable=False),
        sa.Column("north_load_mw", sa.Float(), nullable=False),
        sa.Column("central_generation_mw", sa.Float(), nullable=False),
        sa.Column("central_load_mw", sa.Float(), nullable=False),
        sa.Column("south_generation_mw", sa.Float(), nullable=False),
        sa.Column("south_load_mw", sa.Float(), nullable=False),
        sa.Column("east_generation_mw", sa.Float(), nullable=False),
        sa.Column("east_load_mw", sa.Float(), nullable=False),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint(
            "observed_at",
            name="uq_taipower_area_snapshot_observed_at",
        ),
    )
    op.create_index(
        "ix_taipower_area_snapshot_observed_at",
        "taipower_area_snapshot",
        ["observed_at"],
        unique=False,
    )

    op.create_table(
        "taipower_operating_reserve",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("date", sa.Date(), nullable=False),
        sa.Column("peak_load_mw", sa.Float(), nullable=False),
        sa.Column("reserve_capacity_mw", sa.Float(), nullable=False),
        sa.Column("reserve_rate_percent", sa.Float(), nullable=False),
        sa.Column(
            "is_forecast",
            sa.Boolean(),
            server_default=sa.text("false"),
            nullable=False,
        ),
        sa.Column("published_at", sa.DateTime(timezone=True), nullable=True),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint("date", name="uq_taipower_operating_reserve_date"),
    )
    op.create_index(
        "ix_taipower_operating_reserve_date",
        "taipower_operating_reserve",
        ["date"],
        unique=False,
    )

    op.create_table(
        "taipower_generator",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("published_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("sequence", sa.Integer(), nullable=False),
        sa.Column("category_code", sa.String(length=64), nullable=False),
        sa.Column("category", sa.String(length=160), nullable=False),
        sa.Column("subcategory", sa.String(length=160), nullable=True),
        sa.Column("unit_name", sa.String(length=160), nullable=False),
        sa.Column("installed_capacity_mw", sa.Float(), nullable=True),
        sa.Column("installed_capacity_percent", sa.Float(), nullable=True),
        sa.Column("net_generation_mw", sa.Float(), nullable=True),
        sa.Column("net_generation_percent", sa.Float(), nullable=True),
        sa.Column("utilization_percent", sa.Float(), nullable=True),
        sa.Column("status", sa.String(length=255), nullable=True),
        sa.Column(
            "is_summary",
            sa.Boolean(),
            server_default=sa.text("false"),
            nullable=False,
        ),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint(
            "published_at",
            "sequence",
            name="uq_taipower_generator_snapshot_sequence",
        ),
    )
    op.create_index(
        "ix_taipower_generator_published_at",
        "taipower_generator",
        ["published_at"],
        unique=False,
    )
    op.create_index(
        "ix_taipower_generator_category_code",
        "taipower_generator",
        ["category_code"],
        unique=False,
    )
    op.create_index(
        "ix_taipower_generator_unit_name",
        "taipower_generator",
        ["unit_name"],
        unique=False,
    )


def downgrade() -> None:
    op.drop_index(
        "ix_taipower_generator_unit_name",
        table_name="taipower_generator",
    )
    op.drop_index(
        "ix_taipower_generator_category_code",
        table_name="taipower_generator",
    )
    op.drop_index(
        "ix_taipower_generator_published_at",
        table_name="taipower_generator",
    )
    op.drop_table("taipower_generator")

    op.drop_index(
        "ix_taipower_operating_reserve_date",
        table_name="taipower_operating_reserve",
    )
    op.drop_table("taipower_operating_reserve")

    op.drop_index(
        "ix_taipower_area_snapshot_observed_at",
        table_name="taipower_area_snapshot",
    )
    op.drop_table("taipower_area_snapshot")

    op.drop_index(
        "ix_taipower_area_load_observed_at",
        table_name="taipower_area_load",
    )
    op.drop_table("taipower_area_load")

    op.drop_index(
        "ix_taipower_fuel_mix_observed_at",
        table_name="taipower_fuel_mix",
    )
    op.drop_table("taipower_fuel_mix")

    op.drop_index(
        "ix_taipower_power_snapshot_published_at",
        table_name="taipower_power_snapshot",
    )
    op.drop_table("taipower_power_snapshot")
