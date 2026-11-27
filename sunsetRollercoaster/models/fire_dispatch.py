from datetime import datetime
from typing import Optional

from sqlalchemy import Column, DateTime, Index, String, Text, UniqueConstraint
from sqlmodel import Field, SQLModel


class FireDispatchRecord(SQLModel, table=True):
    """消防局即時案件的每一種狀態各存一筆。

    同一案件換狀態或派遣分隊時新增一筆，其餘欄位相同的狀態只更新 last_seen_at。
    多數縣市不公開案件編號，因此不推斷「同一案件」，保留原始狀態變化。
    """

    __tablename__ = "fire_dispatch_record"
    __table_args__ = (
        # 新北沒有受理時間、多數縣市沒有編號與分隊，NULL 也必須視為相同值。
        UniqueConstraint(
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
        Index("ix_fire_dispatch_record_county_first_seen_at", "county", "first_seen_at"),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    county: str = Field(sa_column=Column(String(32), nullable=False))
    received_at: Optional[datetime] = Field(
        default=None, sa_column=Column(DateTime(timezone=True), nullable=True)
    )
    category: str = Field(sa_column=Column(Text, nullable=False))
    location: str = Field(sa_column=Column(Text, nullable=False))
    unit: Optional[str] = Field(default=None, sa_column=Column(Text, nullable=True))
    status: Optional[str] = Field(default=None, sa_column=Column(Text, nullable=True))
    case_id: Optional[str] = Field(default=None, sa_column=Column(Text, nullable=True))
    latitude: Optional[float] = None
    longitude: Optional[float] = None
    first_seen_at: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
    last_seen_at: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False)
    )
