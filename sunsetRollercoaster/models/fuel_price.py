from datetime import datetime
from typing import Optional

from sqlalchemy import Column, DateTime, UniqueConstraint
from sqlmodel import Field, SQLModel


class NationwideFuelPrice(SQLModel, table=True):
    """經濟部能源署公布的全國汽柴油週均價（新臺幣元／公升）。"""

    __tablename__ = "nationwide_fuel_price"
    __table_args__ = (
        UniqueConstraint(
            "period_start",
            "period_end",
            name="uq_nationwide_fuel_price_period",
        ),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    period_start: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
    period_end: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
    unleaded_92: float
    unleaded_95: float
    unleaded_98: float
    super_diesel: float
