# E2E Test Suite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a full end-to-end test harness: a FastAPI test server, pytest fixtures to manage code-mcp lifecycle, and Python MCP SDK tests that exercise the entire pipeline.

**Architecture:** FastAPI app serves endpoints + OpenAPI spec on a random port. Pytest fixtures spawn `code-mcp run` pointed at the spec URL, connect the Python MCP SDK client over stdio (primary) or HTTP/SSE (secondary). Tests invoke MCP tools and execute Luau scripts, asserting correct responses.

**Tech Stack:** Python 3.11+, FastAPI, uvicorn, mcp (Python SDK), pytest, pytest-asyncio, PyJWT, cryptography, httpx, uv (package manager)

---

### Task 1: Python project scaffold

**Files:**
- Create: `e2e/pyproject.toml`
- Create: `e2e/test_api/__init__.py`
- Create: `e2e/tests/__init__.py`

**Step 1: Create pyproject.toml**

```toml
[project]
name = "code-mcp-e2e"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "fastapi>=0.115.0",
    "uvicorn>=0.34.0",
    "mcp>=1.12.0",
    "pytest>=8.0.0",
    "pytest-asyncio>=0.25.0",
    "PyJWT[crypto]>=2.9.0",
    "httpx>=0.28.0",
]

[tool.pytest.ini_options]
asyncio_mode = "auto"
testpaths = ["tests"]
```

**Step 2: Create empty `__init__.py` files**

Create `e2e/test_api/__init__.py` and `e2e/tests/__init__.py` as empty files.

**Step 3: Install dependencies**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv sync`
Expected: Dependencies resolve and install successfully.

**Step 4: Commit**

```bash
git add e2e/pyproject.toml e2e/uv.lock e2e/test_api/__init__.py e2e/tests/__init__.py
git commit -m "chore: scaffold e2e test Python project"
```

---

### Task 2: Pydantic models and seed data

**Files:**
- Create: `e2e/test_api/models.py`
- Create: `e2e/test_api/seed.py`

**Step 1: Write models**

```python
from enum import Enum
from pydantic import BaseModel


class PetStatus(str, Enum):
    active = "active"
    adopted = "adopted"
    pending = "pending"


class Pet(BaseModel):
    id: int
    name: str
    status: PetStatus
    tag: str | None = None
    owner_id: int | None = None


class PetCreate(BaseModel):
    name: str
    status: PetStatus = PetStatus.active
    tag: str | None = None
    owner_id: int | None = None


class PetUpdate(BaseModel):
    name: str | None = None
    status: PetStatus | None = None
    tag: str | None = None
    owner_id: int | None = None


class PetList(BaseModel):
    items: list[Pet]
    total: int


class Owner(BaseModel):
    id: int
    name: str
    email: str
```

**Step 2: Write seed data**

```python
from test_api.models import Owner, Pet, PetStatus


def seed_pets() -> dict[int, Pet]:
    pets = [
        Pet(id=1, name="Fido", status=PetStatus.active, tag="dog", owner_id=1),
        Pet(id=2, name="Whiskers", status=PetStatus.adopted, tag="cat", owner_id=1),
        Pet(id=3, name="Buddy", status=PetStatus.active, tag="dog", owner_id=2),
        Pet(id=4, name="Luna", status=PetStatus.pending, tag="cat"),
    ]
    return {p.id: p for p in pets}


def seed_owners() -> dict[int, Owner]:
    owners = [
        Owner(id=1, name="Alice", email="alice@example.com"),
        Owner(id=2, name="Bob", email="bob@example.com"),
    ]
    return {o.id: o for o in owners}
```

**Step 3: Commit**

```bash
git add e2e/test_api/models.py e2e/test_api/seed.py
git commit -m "feat(e2e): add Pydantic models and seed data"
```

---

### Task 3: FastAPI auth middleware

**Files:**
- Create: `e2e/test_api/auth.py`

**Step 1: Write auth dependency**

The test API uses two hardcoded credential values. Bearer token on `Authorization` header and API key on `X-Api-Key` header. Read endpoints are public; mutation endpoints require auth.

```python
from fastapi import Depends, HTTPException, Request

BEARER_TOKEN = "test-secret-123"
API_KEY = "test-key-456"


def require_auth(request: Request) -> None:
    auth_header = request.headers.get("Authorization", "")
    api_key_header = request.headers.get("X-Api-Key", "")

    if auth_header == f"Bearer {BEARER_TOKEN}":
        return
    if api_key_header == API_KEY:
        return

    raise HTTPException(status_code=401, detail="Unauthorized")
```

**Step 2: Commit**

```bash
git add e2e/test_api/auth.py
git commit -m "feat(e2e): add test API auth middleware"
```

---

### Task 4: FastAPI app with all endpoints

**Files:**
- Create: `e2e/test_api/app.py`

**Step 1: Write the FastAPI app**

The app title MUST be "Test API" — code-mcp derives the api name as `test_api` from this, and env vars use `TEST_API_` prefix accordingly.

```python
from fastapi import Depends, FastAPI, HTTPException

from test_api.auth import require_auth
from test_api.models import Owner, Pet, PetCreate, PetList, PetStatus, PetUpdate
from test_api.seed import seed_owners, seed_pets

app = FastAPI(
    title="Test API",
    version="1.0.0",
    description="E2E test API for code-mcp",
)

# In-memory state
db_pets: dict[int, Pet] = seed_pets()
db_owners: dict[int, Owner] = seed_owners()
next_pet_id: int = 5


@app.post("/reset", tags=["admin"])
def reset_data() -> dict[str, str]:
    global db_pets, db_owners, next_pet_id
    db_pets = seed_pets()
    db_owners = seed_owners()
    next_pet_id = 5
    return {"status": "ok"}


@app.get("/pets", tags=["pets"])
def list_pets(
    limit: int | None = None,
    status: PetStatus | None = None,
) -> PetList:
    pets = list(db_pets.values())
    if status is not None:
        pets = [p for p in pets if p.status == status]
    total = len(pets)
    if limit is not None:
        pets = pets[:limit]
    return PetList(items=pets, total=total)


@app.post("/pets", tags=["pets"], status_code=201, dependencies=[Depends(require_auth)])
def create_pet(body: PetCreate) -> Pet:
    global next_pet_id
    pet = Pet(id=next_pet_id, **body.model_dump())
    db_pets[next_pet_id] = pet
    next_pet_id += 1
    return pet


@app.get("/pets/{pet_id}", tags=["pets"])
def get_pet(pet_id: int) -> Pet:
    if pet_id not in db_pets:
        raise HTTPException(status_code=404, detail="Pet not found")
    return db_pets[pet_id]


@app.put("/pets/{pet_id}", tags=["pets"], dependencies=[Depends(require_auth)])
def update_pet(pet_id: int, body: PetUpdate) -> Pet:
    if pet_id not in db_pets:
        raise HTTPException(status_code=404, detail="Pet not found")
    existing = db_pets[pet_id]
    updated = existing.model_copy(update=body.model_dump(exclude_unset=True))
    db_pets[pet_id] = updated
    return updated


@app.delete("/pets/{pet_id}", tags=["pets"], dependencies=[Depends(require_auth)])
def delete_pet(pet_id: int) -> dict[str, str]:
    if pet_id not in db_pets:
        raise HTTPException(status_code=404, detail="Pet not found")
    del db_pets[pet_id]
    return {"status": "deleted"}


@app.get("/owners", tags=["owners"])
def list_owners() -> list[Owner]:
    return list(db_owners.values())


@app.get("/owners/{owner_id}/pets", tags=["owners"])
def list_owner_pets(owner_id: int) -> list[Pet]:
    if owner_id not in db_owners:
        raise HTTPException(status_code=404, detail="Owner not found")
    return [p for p in db_pets.values() if p.owner_id == owner_id]
```

**Step 2: Verify the app starts and the OpenAPI spec is correct**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run python -c "from test_api.app import app; import json; print(json.dumps(app.openapi(), indent=2))" | head -40`
Expected: Valid OpenAPI 3.1 JSON with paths for `/pets`, `/pets/{pet_id}`, `/owners`, `/owners/{owner_id}/pets`, `/reset`.

**Step 3: Commit**

```bash
git add e2e/test_api/app.py
git commit -m "feat(e2e): add FastAPI test app with CRUD endpoints"
```

---

### Task 5: Add security schemes to the OpenAPI spec

**Files:**
- Modify: `e2e/test_api/app.py`

FastAPI doesn't auto-generate `securitySchemes` from a plain `Depends` function. We need to use FastAPI's security utilities so the OpenAPI spec includes the auth info that code-mcp needs to wire up credentials.

**Step 1: Update app.py to use FastAPI security schemes**

Replace the `require_auth` import and `Depends(require_auth)` usage with FastAPI security dependencies. Modify `e2e/test_api/auth.py`:

```python
from fastapi import HTTPException
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer, APIKeyHeader

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
```

Wait — this creates a circular import because `Depends` comes from fastapi. The full updated `auth.py`:

```python
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
```

In `app.py`, the imports and usage of `Depends(require_auth)` remain the same — FastAPI resolves the nested `Depends` automatically.

**Step 2: Verify OpenAPI spec now includes securitySchemes**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run python -c "from test_api.app import app; import json; spec = app.openapi(); print(json.dumps(spec.get('components', {}).get('securitySchemes', {}), indent=2))"`
Expected: Output includes `HTTPBearer` and `APIKeyHeader` schemes.

**Step 3: Commit**

```bash
git add e2e/test_api/auth.py
git commit -m "feat(e2e): add OpenAPI security schemes for bearer and API key"
```

---

### Task 6: Session-scoped pytest fixtures

**Files:**
- Create: `e2e/conftest.py`

These fixtures manage the full lifecycle: start test API, build code-mcp, spawn code-mcp process, connect MCP client.

**Step 1: Write conftest.py**

```python
import asyncio
import socket
import subprocess
import sys
import time
from pathlib import Path

import httpx
import pytest
import uvicorn

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CODE_MCP_BINARY = PROJECT_ROOT / "target" / "release" / "code-mcp"


def _free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _wait_for_http(url: str, timeout: float = 10.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            resp = httpx.get(url, timeout=2.0)
            if resp.status_code < 500:
                return
        except httpx.ConnectError:
            pass
        time.sleep(0.1)
    raise TimeoutError(f"Server at {url} did not start in {timeout}s")


@pytest.fixture(scope="session")
def code_mcp_binary() -> Path:
    if not CODE_MCP_BINARY.exists():
        subprocess.run(
            ["cargo", "build", "--release"],
            cwd=PROJECT_ROOT,
            check=True,
        )
    return CODE_MCP_BINARY


@pytest.fixture(scope="session")
def test_api_url() -> str:
    port = _free_port()
    config = uvicorn.Config(
        "test_api.app:app",
        host="127.0.0.1",
        port=port,
        log_level="warning",
    )
    server = uvicorn.Server(config)
    thread = __import__("threading").Thread(target=server.run, daemon=True)
    thread.start()
    url = f"http://127.0.0.1:{port}"
    _wait_for_http(f"{url}/openapi.json")
    yield url
    server.should_exit = True
    thread.join(timeout=5)


@pytest.fixture(scope="session")
def openapi_spec_url(test_api_url: str) -> str:
    return f"{test_api_url}/openapi.json"


@pytest.fixture(autouse=True)
def reset_test_data(test_api_url: str) -> None:
    httpx.post(f"{test_api_url}/reset", timeout=5.0)
```

**Step 2: Verify fixtures load**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest --co -q`
Expected: No errors, no tests collected yet (empty test files).

**Step 3: Commit**

```bash
git add e2e/conftest.py
git commit -m "feat(e2e): add session-scoped pytest fixtures for test API and binary"
```

---

### Task 7: MCP stdio client fixture

**Files:**
- Create: `e2e/tests/conftest.py`

This fixture spawns `code-mcp run <spec_url>` with the correct env vars and connects the Python MCP SDK over stdio.

**Step 1: Write the MCP stdio fixture**

```python
import asyncio
import subprocess
import sys
from pathlib import Path

import pytest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


@pytest.fixture(scope="session")
def mcp_stdio_session(code_mcp_binary: Path, openapi_spec_url: str):
    """Spawn code-mcp and connect an MCP client over stdio."""
    env = {
        "PATH": "/usr/bin:/bin",
        "TEST_API_BEARER_TOKEN": "test-secret-123",
        "TEST_API_API_KEY": "test-key-456",
    }

    async def _run():
        server_params = StdioServerParameters(
            command=str(code_mcp_binary),
            args=["run", openapi_spec_url],
            env=env,
        )
        async with stdio_client(server_params) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                yield session

    # We need to run the async generator in an event loop that persists
    # for the session. Use a dedicated loop in a thread.
    loop = asyncio.new_event_loop()
    gen = _run()
    session = loop.run_until_complete(gen.__anext__())
    yield session
    try:
        loop.run_until_complete(gen.__anext__())
    except StopAsyncIteration:
        pass
    loop.close()
```

Note: the async context manager lifecycle for a session-scoped fixture with async generators is tricky. If `mcp`'s `stdio_client` doesn't work cleanly with this pattern, an alternative is to use `pytest-asyncio`'s `scope="session"` support:

```python
@pytest_asyncio.fixture(scope="session")
async def mcp_stdio_session(code_mcp_binary: Path, openapi_spec_url: str):
    env = {
        "PATH": "/usr/bin:/bin",
        "TEST_API_BEARER_TOKEN": "test-secret-123",
        "TEST_API_API_KEY": "test-key-456",
    }
    server_params = StdioServerParameters(
        command=str(code_mcp_binary),
        args=["run", openapi_spec_url],
        env=env,
    )
    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            yield session
```

Prefer the `pytest_asyncio.fixture` approach — it's cleaner and `pytest-asyncio>=0.25` supports session-scoped async fixtures.

**Step 2: Verify the fixture loads without error**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest --co -q`
Expected: No collection errors.

**Step 3: Commit**

```bash
git add e2e/tests/conftest.py
git commit -m "feat(e2e): add MCP stdio client fixture"
```

---

### Task 8: Tool invocation tests (stdio)

**Files:**
- Create: `e2e/tests/test_stdio_tools.py`

**Step 1: Write a minimal smoke test first**

```python
import pytest
from mcp import ClientSession


@pytest.mark.asyncio
async def test_list_tools(mcp_stdio_session: ClientSession):
    """Verify that the MCP server exposes the expected tools."""
    result = await mcp_stdio_session.list_tools()
    tool_names = {t.name for t in result.tools}
    assert "list_apis" in tool_names
    assert "list_functions" in tool_names
    assert "get_function_docs" in tool_names
    assert "search_docs" in tool_names
    assert "get_schema" in tool_names
    assert "execute_script" in tool_names
```

**Step 2: Run the smoke test**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest tests/test_stdio_tools.py::test_list_tools -v`
Expected: PASS. This proves the entire pipeline works: test API serving spec → code-mcp generating SDK → MCP server running → client connected.

**Step 3: Write remaining tool tests**

```python
@pytest.mark.asyncio
async def test_list_apis(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("list_apis", {})
    text = result.content[0].text
    assert "test_api" in text


@pytest.mark.asyncio
async def test_list_functions(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("list_functions", {})
    text = result.content[0].text
    # Should contain functions derived from our endpoints
    assert "list_pets" in text
    assert "create_pet" in text
    assert "get_pet" in text


@pytest.mark.asyncio
async def test_list_functions_filter_by_tag(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("list_functions", {"tag": "pets"})
    text = result.content[0].text
    assert "list_pets" in text


@pytest.mark.asyncio
async def test_get_function_docs(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("get_function_docs", {"name": "list_pets"})
    text = result.content[0].text
    # Should contain a Luau type signature
    assert "function" in text.lower() or "list_pets" in text


@pytest.mark.asyncio
async def test_search_docs(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("search_docs", {"query": "pet"})
    text = result.content[0].text
    assert "pet" in text.lower()


@pytest.mark.asyncio
async def test_get_schema(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("get_schema", {"name": "Pet"})
    text = result.content[0].text
    assert "name" in text.lower()
    assert "status" in text.lower()
```

**Step 4: Run all tool tests**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest tests/test_stdio_tools.py -v`
Expected: All PASS.

**Step 5: Commit**

```bash
git add e2e/tests/test_stdio_tools.py
git commit -m "test(e2e): add MCP tool invocation tests over stdio"
```

---

### Task 9: Script execution tests (stdio)

**Files:**
- Create: `e2e/tests/test_stdio_scripts.py`

These tests call the `execute_script` tool with Luau scripts and assert the results. The script results come back as JSON text in the tool response.

Important context: code-mcp's `execute_script` tool returns a JSON object with `result`, `logs`, and `stats` fields. The `result` is whatever the Luau script returns.

**Step 1: Write the first script test**

```python
import json

import pytest
from mcp import ClientSession


def parse_result(tool_result) -> dict:
    """Extract the parsed JSON from an execute_script tool response."""
    text = tool_result.content[0].text
    return json.loads(text)


@pytest.mark.asyncio
async def test_list_pets(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'return sdk.list_pets()'
    })
    data = parse_result(result)
    # The seeded data has 4 pets
    assert data["result"] is not None
```

**Step 2: Run to verify basic script execution works**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest tests/test_stdio_scripts.py::test_list_pets -v`
Expected: PASS. If the response structure differs from expected, adjust `parse_result` accordingly — read the actual response text to understand the format.

**Step 3: Write remaining script tests**

Note: The exact Luau function names depend on what code-mcp generates from the OpenAPI spec. The `operationId` values become snake_case function names. From the FastAPI-generated spec, the operationIds will be the Python function names: `list_pets`, `create_pet`, `get_pet`, `update_pet`, `delete_pet`, `list_owners`, `list_owner_pets`. code-mcp converts these to snake_case SDK functions.

Adjust function names in scripts if the generated names differ. Check with `list_functions` tool first.

```python
@pytest.mark.asyncio
async def test_get_pet_by_id(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'return sdk.get_pet({ pet_id = 1 })'
    })
    data = parse_result(result)
    pet = data["result"]
    assert pet["name"] == "Fido"
    assert pet["status"] == "active"


@pytest.mark.asyncio
async def test_create_pet(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": '''
            return sdk.create_pet({
                name = "Spark",
                status = "active",
                tag = "hamster"
            })
        '''
    })
    data = parse_result(result)
    pet = data["result"]
    assert pet["name"] == "Spark"
    assert pet["id"] is not None


@pytest.mark.asyncio
async def test_update_pet(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": '''
            return sdk.update_pet({
                pet_id = 1,
                name = "Fido Jr."
            })
        '''
    })
    data = parse_result(result)
    pet = data["result"]
    assert pet["name"] == "Fido Jr."


@pytest.mark.asyncio
async def test_delete_pet(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'return sdk.delete_pet({ pet_id = 1 })'
    })
    data = parse_result(result)
    assert data["result"]["status"] == "deleted"


@pytest.mark.asyncio
async def test_query_params(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'return sdk.list_pets({ limit = 2, status = "active" })'
    })
    data = parse_result(result)
    items = data["result"]["items"]
    assert len(items) <= 2
    for item in items:
        assert item["status"] == "active"


@pytest.mark.asyncio
async def test_nested_resource(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'return sdk.list_owner_pets({ owner_id = 1 })'
    })
    data = parse_result(result)
    pets = data["result"]
    assert len(pets) > 0
    for pet in pets:
        assert pet["owner_id"] == 1


@pytest.mark.asyncio
async def test_multi_call_script(mcp_stdio_session: ClientSession):
    """Script chains list → get, using output of first call as input to second."""
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": '''
            local all_pets = sdk.list_pets()
            local first_id = all_pets.items[1].id
            local detail = sdk.get_pet({ pet_id = first_id })
            return { list_count = all_pets.total, detail = detail }
        '''
    })
    data = parse_result(result)
    assert data["result"]["list_count"] >= 1
    assert data["result"]["detail"]["name"] is not None


@pytest.mark.asyncio
async def test_create_then_fetch(mcp_stdio_session: ClientSession):
    """Script creates a pet, then fetches it by the returned ID."""
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": '''
            local created = sdk.create_pet({
                name = "Ziggy",
                status = "pending",
                tag = "parrot"
            })
            local fetched = sdk.get_pet({ pet_id = created.id })
            return { created = created, fetched = fetched }
        '''
    })
    data = parse_result(result)
    assert data["result"]["created"]["name"] == "Ziggy"
    assert data["result"]["fetched"]["name"] == "Ziggy"
    assert data["result"]["created"]["id"] == data["result"]["fetched"]["id"]


@pytest.mark.asyncio
async def test_enum_values(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'return sdk.list_pets({ status = "pending" })'
    })
    data = parse_result(result)
    for item in data["result"]["items"]:
        assert item["status"] == "pending"


@pytest.mark.asyncio
async def test_optional_fields(mcp_stdio_session: ClientSession):
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'return sdk.get_pet({ pet_id = 4 })'
    })
    data = parse_result(result)
    pet = data["result"]
    # Pet 4 (Luna) has no owner_id
    assert pet["owner_id"] is None


@pytest.mark.asyncio
async def test_script_error_handling(mcp_stdio_session: ClientSession):
    """Calling a nonexistent pet returns an error, doesn't crash the server."""
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'return sdk.get_pet({ pet_id = 9999 })'
    })
    # Should get an error response but not crash
    text = result.content[0].text
    assert "error" in text.lower() or "404" in text
```

**Step 4: Run all script tests**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest tests/test_stdio_scripts.py -v`
Expected: All PASS.

Troubleshooting: If function names don't match, run `list_functions` first to see what code-mcp generated. FastAPI generates operationIds from the Python function names. The exact parameter names (e.g., `pet_id` vs `petId`) depend on what the OpenAPI spec contains — FastAPI uses the Python parameter name. Adjust Luau scripts accordingly.

**Step 5: Commit**

```bash
git add e2e/tests/test_stdio_scripts.py
git commit -m "test(e2e): add script execution tests with multi-call chaining"
```

---

### Task 10: Auth e2e tests

**Files:**
- Create: `e2e/tests/test_auth.py`

These tests verify upstream API auth behavior: env vars, `_meta.auth` override, and failure cases.

**Step 1: Write auth tests**

For the `_meta.auth` test, we need a separate code-mcp instance WITHOUT the env var set, and pass credentials via `_meta.auth` in the tool call. Since the session-scoped fixture always sets env vars, we'll spawn a fresh process for these tests.

```python
import json

import pytest
import pytest_asyncio
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from pathlib import Path


def parse_result(tool_result) -> dict:
    text = tool_result.content[0].text
    return json.loads(text)


@pytest_asyncio.fixture
async def mcp_no_auth_session(code_mcp_binary: Path, openapi_spec_url: str):
    """code-mcp instance with NO upstream API credentials set."""
    env = {"PATH": "/usr/bin:/bin"}
    server_params = StdioServerParameters(
        command=str(code_mcp_binary),
        args=["run", openapi_spec_url],
        env=env,
    )
    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            yield session


@pytest.mark.asyncio
async def test_no_auth_read_succeeds(mcp_no_auth_session: ClientSession):
    """Public endpoints work without any credentials."""
    result = await mcp_no_auth_session.call_tool("execute_script", {
        "script": 'return sdk.list_pets()'
    })
    data = parse_result(result)
    assert data["result"] is not None


@pytest.mark.asyncio
async def test_no_auth_write_fails(mcp_no_auth_session: ClientSession):
    """Protected endpoints fail without credentials."""
    result = await mcp_no_auth_session.call_tool("execute_script", {
        "script": '''
            return sdk.create_pet({ name = "Fail", status = "active" })
        '''
    })
    text = result.content[0].text
    assert "401" in text or "error" in text.lower() or "Unauthorized" in text


@pytest.mark.asyncio
async def test_meta_auth_override(mcp_no_auth_session: ClientSession):
    """Passing _meta.auth with bearer token allows mutation."""
    result = await mcp_no_auth_session.call_tool("execute_script", {
        "script": '''
            return sdk.create_pet({ name = "Meta", status = "active" })
        ''',
        "_meta": {
            "auth": {
                "test_api": {"type": "bearer", "token": "test-secret-123"}
            }
        }
    })
    data = parse_result(result)
    assert data["result"]["name"] == "Meta"


@pytest.mark.asyncio
async def test_bearer_token_auth(mcp_stdio_session: ClientSession):
    """Env-var bearer token allows mutations."""
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": '''
            return sdk.create_pet({ name = "Bearer", status = "active" })
        '''
    })
    data = parse_result(result)
    assert data["result"]["name"] == "Bearer"
```

Note: API key auth can't easily be tested separately from bearer in the env-var flow because `load_auth_from_env` checks `BEARER_TOKEN` first and stops. To test API key auth, spawn another code-mcp instance with only `TEST_API_API_KEY` set. Add this if needed in a follow-up.

**Step 2: Run auth tests**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest tests/test_auth.py -v`
Expected: All PASS.

**Step 3: Commit**

```bash
git add e2e/tests/test_auth.py
git commit -m "test(e2e): add upstream auth and _meta.auth override tests"
```

---

### Task 11: Execution limit tests

**Files:**
- Modify: `e2e/tests/test_stdio_scripts.py` (add limit tests at the bottom)

These tests need a code-mcp instance with short limits. Add a fixture and tests.

**Step 1: Add limited-instance fixture to `e2e/tests/conftest.py`**

Append to the existing `e2e/tests/conftest.py`:

```python
@pytest_asyncio.fixture
async def mcp_limited_session(code_mcp_binary: Path, openapi_spec_url: str):
    """code-mcp instance with short execution limits."""
    env = {
        "PATH": "/usr/bin:/bin",
        "TEST_API_BEARER_TOKEN": "test-secret-123",
    }
    server_params = StdioServerParameters(
        command=str(code_mcp_binary),
        args=[
            "run", openapi_spec_url,
            "--timeout", "2",
            "--max-api-calls", "3",
        ],
        env=env,
    )
    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            yield session
```

**Step 2: Add limit tests to `e2e/tests/test_stdio_scripts.py`**

Append to the file:

```python
@pytest.mark.asyncio
async def test_script_timeout(mcp_limited_session: ClientSession):
    """An infinite loop is killed after the timeout."""
    result = await mcp_limited_session.call_tool("execute_script", {
        "script": 'while true do end'
    })
    text = result.content[0].text
    assert "timeout" in text.lower() or "time" in text.lower()


@pytest.mark.asyncio
async def test_max_api_calls_exceeded(mcp_limited_session: ClientSession):
    """Script making too many API calls is stopped at the limit."""
    result = await mcp_limited_session.call_tool("execute_script", {
        "script": '''
            for i = 1, 10 do
                sdk.list_pets()
            end
            return "should not reach here"
        '''
    })
    text = result.content[0].text
    assert "api" in text.lower() or "limit" in text.lower() or "exceeded" in text.lower()


@pytest.mark.asyncio
async def test_sandbox_no_file_io(mcp_stdio_session: ClientSession):
    """io.open() is blocked by the Luau sandbox."""
    result = await mcp_stdio_session.call_tool("execute_script", {
        "script": 'local f = io.open("/etc/passwd", "r"); return f'
    })
    text = result.content[0].text
    # Should error — io is not available in the sandbox
    assert "error" in text.lower() or "nil" in text.lower() or "attempt to index" in text.lower()
```

**Step 3: Run limit tests**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest tests/test_stdio_scripts.py -k "timeout or max_api or sandbox" -v`
Expected: All PASS.

**Step 4: Commit**

```bash
git add e2e/tests/conftest.py e2e/tests/test_stdio_scripts.py
git commit -m "test(e2e): add execution limit and sandbox tests"
```

---

### Task 12: HTTP/SSE transport tests

**Files:**
- Create: `e2e/tests/test_http_transport.py`
- Modify: `e2e/tests/conftest.py` (add HTTP fixtures)

**Step 1: Add JWT issuer and HTTP fixtures to `e2e/tests/conftest.py`**

Append:

```python
import json
import time
import jwt
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.backends import default_backend
import threading


@pytest.fixture(scope="session")
def jwt_keys():
    """Generate an RSA key pair for test JWT signing."""
    private_key = rsa.generate_private_key(
        public_exponent=65537,
        key_size=2048,
        backend=default_backend(),
    )
    public_key = private_key.public_key()
    return private_key, public_key


@pytest.fixture(scope="session")
def jwks_server(jwt_keys):
    """Serve a JWKS endpoint with the test public key."""
    from http.server import HTTPServer, BaseHTTPRequestHandler
    import json

    _, public_key = jwt_keys
    # Build JWKS from the public key
    pub_numbers = public_key.public_numbers()

    import base64

    def _int_to_b64(n: int, length: int) -> str:
        return base64.urlsafe_b64encode(
            n.to_bytes(length, byteorder="big")
        ).rstrip(b"=").decode()

    jwks = {
        "keys": [{
            "kty": "RSA",
            "use": "sig",
            "kid": "test-key-1",
            "alg": "RS256",
            "n": _int_to_b64(pub_numbers.n, 256),
            "e": _int_to_b64(pub_numbers.e, 3),
        }]
    }

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(jwks).encode())

        def log_message(self, *args):
            pass  # Suppress logs

    port = _free_port()
    server = HTTPServer(("127.0.0.1", port), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    yield f"http://127.0.0.1:{port}"
    server.shutdown()


@pytest.fixture(scope="session")
def sign_jwt(jwt_keys):
    """Returns a callable that signs a JWT with the test private key."""
    private_key, _ = jwt_keys

    def _sign(audience: str = "test-audience", issuer: str = "test-issuer", exp_seconds: int = 3600) -> str:
        now = int(time.time())
        payload = {
            "sub": "test-user",
            "aud": audience,
            "iss": issuer,
            "iat": now,
            "exp": now + exp_seconds,
        }
        pem = private_key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )
        return jwt.encode(payload, pem, algorithm="RS256", headers={"kid": "test-key-1"})

    return _sign


@pytest_asyncio.fixture
async def mcp_http_session(
    code_mcp_binary: Path,
    openapi_spec_url: str,
    jwks_server: str,
    sign_jwt,
):
    """code-mcp over HTTP/SSE with JWT auth."""
    from mcp.client.streamable_http import streamable_http_client

    port = _free_port()
    env = {
        "PATH": "/usr/bin:/bin",
        "TEST_API_BEARER_TOKEN": "test-secret-123",
    }
    proc = subprocess.Popen(
        [
            str(code_mcp_binary), "run", openapi_spec_url,
            "--transport", "http",
            "--port", str(port),
            "--auth-authority", "test-issuer",
            "--auth-audience", "test-audience",
            "--auth-jwks-uri", f"{jwks_server}/jwks",
        ],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    _wait_for_http(f"http://127.0.0.1:{port}/.well-known/oauth-protected-resource")

    token = sign_jwt()
    headers = {"Authorization": f"Bearer {token}"}
    async with streamable_http_client(
        f"http://127.0.0.1:{port}/mcp",
        headers=headers,
    ) as (read, write, _):
        async with ClientSession(read, write) as session:
            await session.initialize()
            yield session

    proc.terminate()
    proc.wait(timeout=5)


@pytest_asyncio.fixture
async def mcp_http_no_jwt_url(
    code_mcp_binary: Path,
    openapi_spec_url: str,
    jwks_server: str,
):
    """Returns the URL of a code-mcp HTTP server (with auth enabled) for raw HTTP testing."""
    port = _free_port()
    env = {
        "PATH": "/usr/bin:/bin",
        "TEST_API_BEARER_TOKEN": "test-secret-123",
    }
    proc = subprocess.Popen(
        [
            str(code_mcp_binary), "run", openapi_spec_url,
            "--transport", "http",
            "--port", str(port),
            "--auth-authority", "test-issuer",
            "--auth-audience", "test-audience",
            "--auth-jwks-uri", f"{jwks_server}/jwks",
        ],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    _wait_for_http(f"http://127.0.0.1:{port}/.well-known/oauth-protected-resource")
    yield f"http://127.0.0.1:{port}"
    proc.terminate()
    proc.wait(timeout=5)
```

**Step 2: Write HTTP transport tests**

```python
# e2e/tests/test_http_transport.py
import json

import httpx
import pytest
from mcp import ClientSession


def parse_result(tool_result) -> dict:
    text = tool_result.content[0].text
    return json.loads(text)


@pytest.mark.asyncio
async def test_http_list_tools(mcp_http_session: ClientSession):
    result = await mcp_http_session.list_tools()
    tool_names = {t.name for t in result.tools}
    assert "execute_script" in tool_names
    assert "list_apis" in tool_names


@pytest.mark.asyncio
async def test_http_execute_script(mcp_http_session: ClientSession):
    result = await mcp_http_session.call_tool("execute_script", {
        "script": 'return sdk.list_pets()'
    })
    data = parse_result(result)
    assert data["result"] is not None


@pytest.mark.asyncio
async def test_http_auth_required(mcp_http_no_jwt_url: str):
    """Request to /mcp without JWT should be rejected."""
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            f"{mcp_http_no_jwt_url}/mcp",
            json={"jsonrpc": "2.0", "method": "initialize", "id": 1, "params": {}},
            headers={"Content-Type": "application/json"},
        )
    assert resp.status_code == 401


@pytest.mark.asyncio
async def test_http_well_known(mcp_http_no_jwt_url: str):
    """Well-known endpoint returns OAuth metadata."""
    async with httpx.AsyncClient() as client:
        resp = await client.get(
            f"{mcp_http_no_jwt_url}/.well-known/oauth-protected-resource"
        )
    assert resp.status_code == 200
    data = resp.json()
    assert data["resource"] == "test-audience"
    assert "test-issuer" in data["authorization_servers"]
```

**Step 3: Run HTTP transport tests**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest tests/test_http_transport.py -v`
Expected: All PASS.

**Step 4: Commit**

```bash
git add e2e/tests/conftest.py e2e/tests/test_http_transport.py
git commit -m "test(e2e): add HTTP/SSE transport and JWT auth tests"
```

---

### Task 13: Final verification and cleanup

**Step 1: Run the full e2e suite**

Run: `cd /home/alenna/repos/code-mcp/e2e && uv run pytest -v`
Expected: All tests pass (~25 tests).

**Step 2: Add e2e to the project .gitignore if needed**

Check that `e2e/.venv/` and `e2e/__pycache__/` are ignored. Add to `.gitignore` if not already covered:

```
e2e/.venv/
__pycache__/
```

**Step 3: Final commit**

```bash
git add -A
git commit -m "chore(e2e): final cleanup and gitignore updates"
```

---

## Adaptation Notes

Throughout implementation, you will likely need to adapt:

1. **Luau function names**: The exact SDK function names depend on what code-mcp generates from the FastAPI OpenAPI spec. FastAPI uses the Python function name as `operationId`. Run `list_functions` tool early and adjust scripts.

2. **Luau parameter syntax**: Parameters might be positional or table-based depending on how code-mcp's registry binds them. Check `get_function_docs` output for each function.

3. **Response structure**: The `execute_script` tool's response format (`result`, `logs`, `stats` fields) should be verified from the first passing test. Adjust `parse_result` helper accordingly.

4. **OpenAPI spec version**: FastAPI generates OpenAPI 3.1 by default. code-mcp's parser uses `openapiv3` crate which handles 3.0.x. If there are parsing issues, pin FastAPI to generate 3.0 with `app = FastAPI(..., openapi_version="3.0.3")` in `app.py`.

5. **`_meta` passing**: The Python MCP SDK may or may not support passing `_meta` in `call_tool` arguments directly. If not, the `_meta.auth` test needs adjustment — possibly by including `_meta` in the JSON-RPC params directly.

6. **`pytest-asyncio` session scope**: Requires `asyncio_mode = "auto"` in pytest config and `pytest-asyncio>=0.25`. If async session fixtures don't work, fall back to sync fixtures with `asyncio.run()`.
