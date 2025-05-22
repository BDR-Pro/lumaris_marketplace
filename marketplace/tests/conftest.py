from unittest.mock import patch

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from admin_api.database import get_db
from admin_api.models import Base

# ✅ Patch to skip production DB creation
with patch("admin_api.main.Base.metadata.create_all"):
    from admin_api.main import create_app


@pytest.fixture()
def client():
    # Create a shared in-memory database connection
    test_engine = create_engine(
        "sqlite:///:memory:", connect_args={"check_same_thread": False}
    )
    connection = test_engine.connect()

    # Begin a nested transaction (optional but useful for test isolation)
    transaction = connection.begin()

    # Bind a sessionmaker to the connection
    TestingSessionLocal = sessionmaker(bind=connection)

    # Create the schema once
    Base.metadata.create_all(bind=connection)

    def override_get_db():
        db = TestingSessionLocal()
        try:
            yield db
        finally:
            db.close()

    app = create_app()
    app.dependency_overrides[get_db] = override_get_db
    test_client = TestClient(app)

    yield test_client

    # Cleanup: Rollback the transaction and dispose the connection
    transaction.rollback()
    connection.close()
