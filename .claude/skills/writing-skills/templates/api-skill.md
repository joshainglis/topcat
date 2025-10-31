---
name: integrating-apiname
description: Integrates with [API name] for [primary functions]. Use when [accessing data], [performing operations], or [other use cases].
---

# Integrating [API Name]

## Setup

```python
# Installation
pip install api-client

# Authentication
from api_client import Client

client = Client(
    api_key="your-key",
    base_url="https://api.example.com"
)
```

## Common Operations

### GET Resources

```python
# List resources
resources = client.resources.list(limit=10)

# Get specific resource
resource = client.resources.get("resource-id")
```

### POST Data

```python
# Create new resource
new_resource = client.resources.create({
    "name": "Example",
    "type": "standard"
})
```

### Error Handling

```python
from api_client.exceptions import APIError

try:
    result = client.resources.get("id")
except APIError as e:
    print(f"Error {e.status_code}: {e.message}")
```

## Authentication Methods

- **API Key**: Set in headers or query params
- **OAuth**: See [reference/oauth.md](reference/oauth.md)
- **JWT**: See [reference/jwt.md](reference/jwt.md)

## Rate Limiting

Handle rate limits gracefully:

```python
import time

def with_retry(func, max_retries=3):
    for i in range(max_retries):
        try:
            return func()
        except RateLimitError as e:
            time.sleep(e.retry_after)
    raise Exception("Max retries exceeded")
```

## Advanced Usage

**Pagination**: See [reference/pagination.md](reference/pagination.md)
**Webhooks**: See [reference/webhooks.md](reference/webhooks.md)
**Batch Operations**: See [reference/batch.md](reference/batch.md)