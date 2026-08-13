from datetime import date as _date
from datetime import datetime
from typing import Optional

from sqlalchemy import Boolean, Column, Date, DateTime, String, UniqueConstraint
from sqlmodel import Field, SQLModel


class TaipowerPowerSnapshot(SQLModel, table=True):
    """台電今日電力資訊快照；所有電力欄位皆為 MW。"""

    __tablename__ = "taipower_power_snapshot"
    __table_args__ = (
        UniqueConstraint(
            "published_at",
            name="uq_taipower_power_snapshot_published_at",
        ),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    published_at: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
    current_load_mw: Optional[float] = None
    current_utilization_percent: Optional[float] = None
    forecast_max_supply_mw: Optional[float] = None
    forecast_peak_demand_mw: Optional[float] = None
    forecast_peak_reserve_mw: Optional[float] = None
    forecast_peak_reserve_rate_percent: Optional[float] = None
    forecast_peak_reserve_indicator: Optional[str] = Field(
        default=None,
        sa_column=Column(String(16), nullable=True),
    )
    forecast_peak_hour_range: Optional[str] = Field(
        default=None,
        sa_column=Column(String(32), nullable=True),
    )
    yesterday_date: Optional[_date] = Field(
        default=None,
        sa_column=Column(Date, nullable=True),
    )
    yesterday_max_supply_mw: Optional[float] = None
    yesterday_peak_demand_mw: Optional[float] = None
    yesterday_peak_reserve_mw: Optional[float] = None
    yesterday_peak_reserve_rate_percent: Optional[float] = None
    yesterday_peak_reserve_indicator: Optional[str] = Field(
        default=None,
        sa_column=Column(String(16), nullable=True),
    )
    realtime_max_supply_mw: Optional[float] = None
    realtime_peak_at: Optional[datetime] = Field(
        default=None,
        sa_column=Column(DateTime(timezone=True), nullable=True),
    )


class TaipowerFuelMix(SQLModel, table=True):
    """今日用電曲線（依燃料類別）；所有電力欄位皆為 MW。"""

    __tablename__ = "taipower_fuel_mix"
    __table_args__ = (
        UniqueConstraint(
            "observed_at",
            name="uq_taipower_fuel_mix_observed_at",
        ),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    observed_at: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
    lng_mw: float
    ipp_lng_mw: float
    coal_mw: float
    ipp_coal_mw: float
    cogeneration_mw: float
    fuel_oil_mw: float
    solar_mw: float
    wind_mw: float
    hydro_mw: float
    energy_storage_mw: float
    other_renewable_mw: float
    energy_storage_load_mw: float
    total_mw: float


class TaipowerAreaLoad(SQLModel, table=True):
    """今日用電曲線（依區域分類）；所有電力欄位皆為 MW。"""

    __tablename__ = "taipower_area_load"
    __table_args__ = (
        UniqueConstraint(
            "observed_at",
            name="uq_taipower_area_load_observed_at",
        ),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    observed_at: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
    north_load_mw: float
    central_load_mw: float
    south_load_mw: float
    east_load_mw: float
    total_load_mw: float


class TaipowerAreaSnapshot(SQLModel, table=True):
    """各區即時發電與用電快照；所有電力欄位皆為 MW。"""

    __tablename__ = "taipower_area_snapshot"
    __table_args__ = (
        UniqueConstraint(
            "observed_at",
            name="uq_taipower_area_snapshot_observed_at",
        ),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    observed_at: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
    north_generation_mw: float
    north_load_mw: float
    central_generation_mw: float
    central_load_mw: float
    south_generation_mw: float
    south_load_mw: float
    east_generation_mw: float
    east_load_mw: float


class TaipowerOperatingReserve(SQLModel, table=True):
    """每日尖峰負載與備轉容量；所有電力欄位皆為 MW。"""

    __tablename__ = "taipower_operating_reserve"
    __table_args__ = (
        UniqueConstraint("date", name="uq_taipower_operating_reserve_date"),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    date: _date = Field(sa_column=Column(Date, nullable=False, index=True))
    peak_load_mw: float
    reserve_capacity_mw: float
    reserve_rate_percent: float
    is_forecast: bool = Field(
        default=False,
        sa_column=Column(Boolean, nullable=False, server_default="false"),
    )
    published_at: Optional[datetime] = Field(
        default=None,
        sa_column=Column(DateTime(timezone=True), nullable=True),
    )


class TaipowerGenerator(SQLModel, table=True):
    """台電系統各機組發電量快照；電力欄位皆為 MW。"""

    __tablename__ = "taipower_generator"
    __table_args__ = (
        UniqueConstraint(
            "published_at",
            "sequence",
            name="uq_taipower_generator_snapshot_sequence",
        ),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    published_at: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
    sequence: int
    category_code: str = Field(sa_column=Column(String(64), nullable=False, index=True))
    category: str = Field(sa_column=Column(String(160), nullable=False))
    subcategory: Optional[str] = Field(
        default=None,
        sa_column=Column(String(160), nullable=True),
    )
    unit_name: str = Field(sa_column=Column(String(160), nullable=False, index=True))
    installed_capacity_mw: Optional[float] = None
    installed_capacity_percent: Optional[float] = None
    net_generation_mw: Optional[float] = None
    net_generation_percent: Optional[float] = None
    utilization_percent: Optional[float] = None
    status: Optional[str] = Field(
        default=None,
        sa_column=Column(String(255), nullable=True),
    )
    is_summary: bool = Field(
        default=False,
        sa_column=Column(Boolean, nullable=False, server_default="false"),
    )
