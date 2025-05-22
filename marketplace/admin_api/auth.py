"""Authentication endpoints using OAuth2.

Provides a dummy login route for demonstration or testing purposes.
"""

from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.security import OAuth2PasswordBearer, OAuth2PasswordRequestForm

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
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Invalid credentials"
    )
