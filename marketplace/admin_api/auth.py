"""
Authentication module for the Lumaris Marketplace API.
"""

from datetime import datetime, timedelta
from typing import Optional

from fastapi import Depends, HTTPException, status, APIRouter
from fastapi.security import OAuth2PasswordBearer, OAuth2PasswordRequestForm
from jose import JWTError, jwt
from sqlalchemy.orm import Session

from .database import get_db
from .models import Node, NodeToken

# Secret key for JWT token
# In production, this should be stored securely
SECRET_KEY = "09d25e094faa6ca2556c818166b7a9563b93f7099f6f0f4caa6cf63b88e8d3e7"
ALGORITHM = "HS256"
ACCESS_TOKEN_EXPIRE_MINUTES = 60 * 24 * 7  # 1 week

# Create router
router = APIRouter()

# OAuth2 scheme for token authentication
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="token")

@router.post("/token")
async def login(form_data: OAuth2PasswordRequestForm = Depends()):
    """
    Authenticate a user and return a token.
    This is a dummy endpoint for demonstration purposes.
    """
    # This is a dummy implementation
    if form_data.username != "admin" or form_data.password != "admin":
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Incorrect username or password",
            headers={"WWW-Authenticate": "Bearer"},
        )
    
    # Return a dummy token
    return {"access_token": "dummy_token", "token_type": "bearer"}

def verify_api_key(api_key: str = Header(..., alias="X-API-Key")):
    """
    Verify the API key.
    This is a dummy implementation for demonstration purposes.
    """
    # This is a dummy implementation
    if api_key != "test-api-key":
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid API key",
        )
    return True

def verify_node_token(token: str, db: Session = Depends(get_db)):
    """
    Verify a node token.
    """
    token_entry = db.query(NodeToken).filter(NodeToken.token == token).first()
    if not token_entry or token_entry.expires_at < datetime.utcnow():
        raise HTTPException(status_code=403, detail="Invalid or expired token")
    return token_entry.node_id

def create_node_token(node_id: str) -> str:
    """
    Create a JWT token for a node.
    
    Args:
        node_id: The ID of the node
    
    Returns:
        str: The JWT token
    """
    expire = datetime.utcnow() + timedelta(minutes=ACCESS_TOKEN_EXPIRE_MINUTES)
    to_encode = {"sub": node_id, "exp": expire}
    encoded_jwt = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    return encoded_jwt

def get_current_node(token: str = Depends(oauth2_scheme), db: Session = Depends(get_db)) -> Node:
    """
    Get the current node from the token.
    
    Args:
        token: The JWT token
        db: The database session
    
    Returns:
        Node: The node
    
    Raises:
        HTTPException: If the token is invalid or the node is not found
    """
    credentials_exception = HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        node_id: str = payload.get("sub")
        if node_id is None:
            raise credentials_exception
    except JWTError:
        raise credentials_exception
    
    node = db.query(Node).filter(Node.id == node_id).first()
    if node is None:
        raise credentials_exception
    
    return node

def get_current_node_from_token(token: str, db: Session = Depends(get_db)) -> Optional[Node]:
    """
    Get the current node from a token string.
    
    Args:
        token: The JWT token
        db: The database session
    
    Returns:
        Optional[Node]: The node, or None if the token is invalid
    """
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        node_id: str = payload.get("sub")
        if node_id is None:
            return None
        
        node = db.query(Node).filter(Node.id == node_id).first()
        return node
    except JWTError:
        return None

