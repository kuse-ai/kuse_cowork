# MCP Protocol

MCP (Model Context Protocol) allows Kuse Cowork to connect to external tool providers, extending the agent's capabilities.

## What is MCP?

MCP is a protocol that enables AI agents to discover and use tools from external servers. It provides:

- **Dynamic Tool Discovery**: Servers advertise available tools
- **Standardized Interface**: Common format for tool invocation
- **Secure Authentication**: OAuth and token-based auth

## Connecting MCP Servers

### Adding a Server

1. Open **Settings** → **MCP**
2. Click **Add Server**
3. Enter server details:

```json
{
  "name": "my-mcp-server",
  "url": "http://localhost:3000",
  "transport": "http"
}
```

4. Configure authentication if required
5. Click **Connect**

### Server Configuration

| Field | Description | Required |
|-------|-------------|----------|
| `name` | Display name | Yes |
| `url` | Server URL | Yes |
| `transport` | `http` or `stdio` | Yes |
| `auth` | Authentication config | No |

### Authentication Types

#### No Authentication

```json
{
  "name": "local-server",
  "url": "http://localhost:3000",
  "transport": "http"
}
```

#### Bearer Token

```json
{
  "name": "api-server",
  "url": "https://api.example.com",
  "transport": "http",
  "auth": {
    "type": "bearer",
    "token": "your-api-token"
  }
}
```

#### OAuth

```json
{
  "name": "oauth-server",
  "url": "https://service.example.com",
  "transport": "http",
  "auth": {
    "type": "oauth",
    "clientId": "your-client-id",
    "authUrl": "https://service.example.com/oauth/authorize",
    "tokenUrl": "https://service.example.com/oauth/token"
  }
}
```

## Using MCP Tools

Once connected, MCP tools appear automatically in the agent's tool list.

### Tool Naming

MCP tools are prefixed with server identifier:

```
mcp_<server_id>_<tool_name>
```

Example:
```
mcp_github_create_issue
mcp_slack_send_message
mcp_database_query
```

### Tool Discovery

The agent automatically discovers tools when:

1. Server connects successfully
2. Server status changes to "Connected"
3. Tools are listed in the agent's context

### Tool Invocation

```json
{
  "name": "mcp_github_create_issue",
  "input": {
    "repo": "owner/repo",
    "title": "Bug Report",
    "body": "Description of the bug"
  }
}
```

## Server Status

### Status Indicators

| Status | Description |
|--------|-------------|
| 🟢 Connected | Server is responsive |
| 🟡 Connecting | Connection in progress |
| 🔴 Disconnected | Server unreachable |
| ⚠️ Error | Configuration issue |

### Connection Management

- **Reconnect**: Retry failed connections
- **Disconnect**: Temporarily disable server
- **Remove**: Delete server configuration

## Building MCP Servers

### Server Requirements

MCP servers must implement:

1. **Tool List Endpoint**: Return available tools
2. **Tool Call Endpoint**: Execute tool invocations
3. **Health Check**: Confirm server status

### Example Server (Python)

```python
from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

# Tool definitions
TOOLS = [
    {
        "name": "hello_world",
        "description": "Returns a greeting",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        }
    }
]

@app.get("/tools")
def list_tools():
    return {"tools": TOOLS}

@app.post("/tools/call")
def call_tool(request: dict):
    tool_name = request["name"]
    args = request["arguments"]

    if tool_name == "hello_world":
        return {"result": f"Hello, {args['name']}!"}

    return {"error": f"Unknown tool: {tool_name}"}

@app.get("/health")
def health():
    return {"status": "ok"}
```

### Example Server (Node.js)

```javascript
const express = require('express');
const app = express();
app.use(express.json());

const tools = [
  {
    name: 'calculate',
    description: 'Performs calculations',
    inputSchema: {
      type: 'object',
      properties: {
        expression: { type: 'string' }
      },
      required: ['expression']
    }
  }
];

app.get('/tools', (req, res) => {
  res.json({ tools });
});

app.post('/tools/call', (req, res) => {
  const { name, arguments: args } = req.body;

  if (name === 'calculate') {
    const result = eval(args.expression); // Use safe-eval in production!
    return res.json({ result: result.toString() });
  }

  res.status(404).json({ error: `Unknown tool: ${name}` });
});

app.get('/health', (req, res) => {
  res.json({ status: 'ok' });
});

app.listen(3000);
```

## Protocol Specification

### Tool List Response

```json
{
  "tools": [
    {
      "name": "tool_name",
      "description": "What the tool does",
      "inputSchema": {
        "type": "object",
        "properties": {
          "param1": {
            "type": "string",
            "description": "Parameter description"
          }
        },
        "required": ["param1"]
      }
    }
  ]
}
```

### Tool Call Request

```json
{
  "name": "tool_name",
  "arguments": {
    "param1": "value1"
  }
}
```

### Tool Call Response

Success:
```json
{
  "result": "Tool execution result"
}
```

Error:
```json
{
  "error": "Error message"
}
```

## Use Cases

### External API Integration

Connect to external services:

- **GitHub**: Create issues, PRs, manage repos
- **Slack**: Send messages, manage channels
- **Jira**: Create tickets, update status
- **Databases**: Query and modify data

### Custom Tooling

Build project-specific tools:

- **Deployment**: Trigger CI/CD pipelines
- **Monitoring**: Check service health
- **Documentation**: Generate docs from code

### Enterprise Integration

Connect to internal services:

- **Internal APIs**: Access proprietary systems
- **Data Sources**: Query internal databases
- **Automation**: Trigger internal workflows

## Security Considerations

### Network Security

::: warning Local Servers Only
    Only connect to trusted MCP servers. Malicious servers can execute arbitrary actions.

### Token Management

- Store tokens securely
- Use short-lived tokens when possible
- Rotate tokens regularly

### OAuth Security

- Verify OAuth redirect URLs
- Use PKCE for public clients
- Validate token scopes

## Troubleshooting

### Server not connecting

    1. Verify URL is correct
    2. Check server is running
    3. Confirm network accessibility
    4. Review authentication config

### Tools not appearing

    1. Check server status is "Connected"
    2. Verify `/tools` endpoint returns valid response
    3. Restart Kuse Cowork

### Tool calls failing

    1. Check tool input matches schema
    2. Review server logs
    3. Verify authentication is valid

### OAuth flow not completing

    1. Check redirect URL configuration
    2. Verify client ID/secret
    3. Confirm token endpoint is accessible

## Configuration Storage

MCP server configurations are stored in:

```
~/.kuse-cowork/settings.db
```

Table: `mcp_servers`

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | Unique identifier |
| name | TEXT | Display name |
| config | JSON | Server configuration |
| status | TEXT | Connection status |

## Next Steps

- [Tools Reference](tools.md) - Built-in tools
- [Agent System](agent.md) - How MCP tools are used
- [Development Guide](../development/setup.md) - Building MCP servers
