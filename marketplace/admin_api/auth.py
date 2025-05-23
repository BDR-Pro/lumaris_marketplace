"""Authentication endpoints using OAuth2.

Provides a dummy login route for demonstration or testing purposes.
"""

from datetime import datetime

from admin_api.database import get_db
from admin_api.models import NodeToken
from fastapi import APIRouter, Depends, Header, HTTPException, status
from fastapi.security import OAuth2PasswordBearer, OAuth2PasswordRequestForm
from sqlalchemy.orm import Session

router = APIRouter()
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="token")


@router.post("/token")
async def login(form_data: OAuth2PasswordRequestForm = Depends()):
    """
    Handle user login with OAuth2 credentials.

    Returns a fake token if username and password match hardcoded values.

    Args:
        form_data (OAuth2PasswordRequestForm): Form with username/password.

    Returns:
        dict: Access token and token type if credentials are valid.

    Raises:
        HTTPException: If credentials are invalid.
    """
    if form_data.username == "admin" and form_data.password == "admin":
        return {"access_token": "fake-admin-token", "token_type": "bearer"}

    raise HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid credentials"
    )


def verify_token(authorization: str = Header(...), db: Session = Depends(get_db)):
    """Verify the provided token from the Authorization header.
    Args:
        authorization (str): The Authorization header value.
        db (Session): Database session.
    Returns:
        str: The node ID associated with the token.
    Raises:
        HTTPException: If the token is invalid or expired.
    """
    if not authorization.startswith("Bearer "):
        raise HTTPException(status_code=403, detail="Invalid auth format")
    token = authorization.split(" ")[1]
    token_entry = db.query(NodeToken).filter_by(token=token).first()
    if not token_entry or token_entry.expires_at < datetime.utcnow():
        raise HTTPException(status_code=403, detail="Invalid or expired token")
    return token_entry.node_id
