"""Pytest configuration for Admin API tests using an in-memory SQLite DB."""

from unittest.mock import patch

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from admin_api.database import get_db
from admin_api.models import Base

# Patch app startup to skip real DB initialization
with patch("admin_api.main.Base.metadata.create_all"):
    from admin_api.main import create_app


@pytest.fixture()
def client():
    """Fixture that provides a FastAPI TestClient with a fresh in-memory database."""
    test_engine = create_engine(
        "sqlite:///:memory:", connect_args={"check_same_thread": False}
    )
    connection = test_engine.connect()
    transaction = connection.begin()

    testing_session_local = sessionmaker(bind=connection)
    Base.metadata.create_all(bind=connection)

    def override_get_db():
        """Yield a session bound to the in-memory DB."""
        db = testing_session_local()
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
