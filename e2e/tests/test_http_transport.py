import json

import httpx
import pytest
from mcp import ClientSession


def parse_result(result) -> dict:
    text = result.content[0].text
    assert not result.isError, f"Script execution error: {text}"
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
        "script": "return sdk.list_pets()"
    })
    data = parse_result(result)
    assert data["result"]["total"] == 4


@pytest.mark.asyncio
async def test_http_auth_required(mcp_http_url: str):
    """Request to /mcp without JWT should be rejected."""
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            f"{mcp_http_url}/mcp",
            json={"jsonrpc": "2.0", "method": "initialize", "id": 1, "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }},
            headers={"Content-Type": "application/json"},
        )
    assert resp.status_code == 401


@pytest.mark.asyncio
async def test_http_well_known(mcp_http_url: str):
    """Well-known endpoint returns OAuth metadata (accessible without auth)."""
    async with httpx.AsyncClient() as client:
        resp = await client.get(f"{mcp_http_url}/.well-known/oauth-protected-resource")
    assert resp.status_code == 200
    data = resp.json()
    assert data["resource"] == "test-audience"
    assert "test-issuer" in data["authorization_servers"]
