from datetime import datetime, date
from typing import List
from sqlmodel import Field, SQLModel


class Invoice(SQLModel, table=True):
    id: int = Field(primary_key=True)
    # only yyyy/mm
    date: date  = Field(default=datetime.now().date())
    # invoice number
    special_prize: int = Field(default=0)
    grand_prize: int    = Field(default=0)  
    first_prize: List[int] = Field(default_factory=list)
    