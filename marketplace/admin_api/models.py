from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import Column, Integer, String, Float

Base = declarative_base()

class Node(Base):
    __tablename__ = 'nodes'
    id = Column(Integer, primary_key=True, index=True)
    hostname = Column(String)
    status = Column(String)
    cpu_usage = Column(Float)
    mem_usage = Column(Float)

class Job(Base):
    __tablename__ = 'jobs'
    id = Column(Integer, primary_key=True, index=True)
    node_id = Column(Integer)
    status = Column(String)
    duration = Column(Float)
    cost = Column(Float)
