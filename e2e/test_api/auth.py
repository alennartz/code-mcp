from fastapi import Depends, HTTPException
from fastapi.security import APIKeyHeader, HTTPAuthorizationCredentials, HTTPBearer

BEARER_TOKEN = "test-secret-123"
API_KEY = "test-key-456"

bearer_scheme = HTTPBearer(auto_error=False)
api_key_scheme = APIKeyHeader(name="X-Api-Key", auto_error=False)


def require_auth(
    bearer: HTTPAuthorizationCredentials | None = Depends(bearer_scheme),
    api_key: str | None = Depends(api_key_scheme),
) -> None:
    if bearer is not None and bearer.credentials == BEARER_TOKEN:
        return
    if api_key is not None and api_key == API_KEY:
        return
    raise HTTPException(status_code=401, detail="Unauthorized")
