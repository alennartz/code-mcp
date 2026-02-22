# Vision

## The Problem

AI agents using MCP tools over complex APIs are wasteful. Each API call is a separate tool invocation, and the LLM must round-trip all intermediate state — even when no judgment is needed. The LLM becomes an expensive, slow data shuttle between function outputs and inputs.

## The Insight

The next evolution of MCP is to give the LLM a scripting runtime instead of individual tool wrappers. The LLM writes a script that orchestrates multiple API calls, sends it for execution, and gets back the result. One round-trip instead of many.

## The Solution

A tool that takes one or more OpenAPI-compliant specifications and generates a container running an MCP server. That server provides:

- **Documentation tools** — the LLM can explore the auto-generated SDK to understand what's available before writing code
- **A Lua scripting runtime** — the LLM sends scripts to execute against the API, not individual tool calls
- **An auto-generated SDK** — strongly-typed Lua bindings derived directly from the OpenAPI spec, giving the LLM a clean, minimal interface to the underlying APIs

## The Stack

- **Rust** — the MCP server process
- **Lua** — the scripting language for agent-authored code
- **OpenAPI** — the input format that drives SDK generation

## The Workflow

1. Feed the tool one or more OpenAPI specs
2. It generates a container image with the MCP server, Lua runtime, and SDK
3. An AI agent connects to the MCP server
4. The agent uses documentation tools to understand the available SDK
5. The agent writes Lua scripts that chain multiple API calls together
6. The agent sends scripts to the MCP server for execution
7. The agent gets back the final result — one round-trip, not dozens
