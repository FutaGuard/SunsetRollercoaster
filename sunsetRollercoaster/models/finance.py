from datetime import datetime
from typing import Optional

from sqlalchemy import Column, DateTime, String, UniqueConstraint
from sqlmodel import Field, SQLModel


class FinanceExchangeRate(SQLModel, table=True):
    """GOOGLEFINANCE 即時匯率快照：1 單位 base 可兌換的 quote 金額。"""

    __tablename__ = "finance_exchange_rate"
    __table_args__ = (
        UniqueConstraint(
            "base",
            "quote",
            "observed_at",
            name="uq_finance_exchange_rate_pair_observed_at",
        ),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    base: str = Field(sa_column=Column(String(3), nullable=False))
    quote: str = Field(sa_column=Column(String(3), nullable=False))
    rate: float
    observed_at: datetime = Field(
        sa_column=Column(DateTime(timezone=True), nullable=False, index=True)
    )
