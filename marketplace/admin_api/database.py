"""Database setup and session management for the admin API."""

from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

SQLALCHEMY_DATABASE_URL = "sqlite:///./admin.db"

# SQLite engine setup with multithreaded access support
engine = create_engine(
    SQLALCHEMY_DATABASE_URL,
    connect_args={"check_same_thread": False}
)

SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


def get_db():
    """
    Dependency that yields a SQLAlchemy session.

    This ensures each request gets its own database session and it's closed properly.
    """
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
