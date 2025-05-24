"""Pytest configuration for Admin API tests using an in-memory SQLite DB."""

# Standard library
from unittest.mock import patch

# Third-party packages
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

# Internal application imports
from admin_api.database import get_db
from admin_api.models import Base
from admin_api.main import create_app


@pytest.fixture()
def client():
    """Provides a TestClient with an in-memory database for isolated testing."""
    test_engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False}
    )
    connection = test_engine.connect()
    transaction = connection.begin()

    test_session = sessionmaker(bind=connection)
    Base.metadata.create_all(bind=connection)

    def override_get_db():
        db = test_session()
        try:
            yield db
        finally:
            db.close()

    app = create_app()
    app.dependency_overrides[get_db] = override_get_db
    test_client = TestClient(app)

    yield test_client

    transaction.rollback()
    connection.close()
