from datetime import datetime
from typing import Optional

from sqlalchemy import Column, DateTime
from sqlmodel import Field, SQLModel


class Reservoir(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    name: Optional[str] = Field(default=None, index=True)
    capavailable: Optional[float] = Field(default=None)
    statisticTimeS: Optional[datetime] = Field(default=None, sa_column=Column(DateTime(timezone=True)))
    statisticTimeE: Optional[datetime] = Field(default=None, sa_column=Column(DateTime(timezone=True)))
    rainFall: Optional[float] = Field(default=None)
    inFlow: Optional[float] = Field(default=None)
    outFlow: Optional[float] = Field(default=None)
    waterlevediff: Optional[float] = Field(default=None)
    recordTime: Optional[datetime] = Field(default=None, sa_column=Column(DateTime(timezone=True), index=True))
    caplevel: Optional[float] = Field(default=None)
    currcap: Optional[float] = Field(default=None)
    currcapper: Optional[float] = Field(default=None)
