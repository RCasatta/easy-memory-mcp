# Easy Memory MCP

A simple MCP (Model Context Protocol) server written in Rust that enables AI assistants to remember user preferences and information across conversations.

## What it does

Provides two tools for AI assistants:
- **add_memory** - Store user preferences, facts, and information
- **get_memories** - Retrieve all stored memories

Memories are persisted to a `memories.md` file with timestamps in a human-readable markdown format.

## Usage

The easiest way is using nix, add the following to the `mcp.json`

```json
{
  "mcpServers": {
    "easy-memory-mcp": {
      "command": "nix",
      "args": [
        "run",
        "github:RCasatta/easy-memory-mcp?rev=24dd20affe2a9a743f4dee1e991d4f13b38ef1f1"
      ]
    }
  }
}
```