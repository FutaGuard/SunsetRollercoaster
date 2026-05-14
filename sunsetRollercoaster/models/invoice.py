from datetime import date as _date, datetime
from typing import List, Optional

from sqlalchemy import JSON, Column
from sqlmodel import Field, SQLModel


class Invoice(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    date: _date = Field(default_factory=lambda: datetime.now().date(), index=True)
    special_prize: int = Field(default=0)
    grand_prize: int = Field(default=0)
    first_prize: List[int] = Field(default_factory=list, sa_column=Column(JSON))
