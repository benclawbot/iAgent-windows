---
name: fullstack-dev
description: Full-stack backend architecture and frontend-backend integration patterns. Reference skill — use when the user is building a full-stack app, creating a REST API with a frontend, scaffolding a backend service, designing service layers, implementing error handling, managing config/auth, setting up API clients, handling file uploads, adding real-time features (SSE/WebSocket), or hardening for production. Knowledge-only — no scripts. Triggers: full-stack app, REST API, backend service, Express, Next.js API, Node.js backend, Python backend, Go backend, design service layer, error handling, auth flow, file upload, real-time, SSE, WebSocket.
implementation-status: implemented
uses-tool: knowledge
tool-actions: []
triggers:
  - "build a full-stack app"
  - "scaffold a backend"
  - "create a REST API"
  - "design service layer"
  - "implement auth flow"
  - "add real-time features"
  - "websocket setup"
  - "sse setup"
  - "production hardening"
  - "twelve-factor app"
allowed-tools: read
platforms: windows,linux,macos
license: MIT
metadata:
  version: "1.0.0"
  category: full-stack
  source: https://github.com/MiniMax-AI/skills/tree/main/skills/fullstack-dev
  references:
    - "The Twelve-Factor App (12factor.net)"
    - "Clean Architecture (Robert C. Martin)"
    - "Domain-Driven Design (Eric Evans)"
    - "Patterns of Enterprise Application Architecture (Martin Fowler)"
    - "Martin Fowler (Testing Pyramid, Contract Tests)"
    - "Google SRE Handbook (Release Engineering)"
    - "ThoughtWorks Technology Radar"
---

# Full-Stack Development

Architecture and integration patterns for full-stack apps. Pure knowledge — no scripts, no external tools. Use as a reference when scaffolding or reviewing a full-stack codebase.

## Do NOT Use This For

- Pure frontend UI/CSS work
- Pure styling/design tasks
- Database schema design in isolation

## Quick Reference

| Topic | Reference |
|---|---|
| API design (REST, GraphQL, gRPC) | `references/api-design.md` |
| Auth flow (JWT, OAuth, session) | `references/auth-flow.md` |
| DB schema & migrations | `references/db-schema.md` |
| Django best practices | `references/django-best-practices.md` |
| Environment / secrets / config | `references/environment-management.md` |
| Release checklist | `references/release-checklist.md` |
| Tech selection (language, framework) | `references/technology-selection.md` |
| Testing strategy (unit, integration, e2e) | `references/testing-strategy.md` |

## Core Patterns

### Service Layer

```ts
class OrderService {
  constructor(private repo: OrderRepository, private email: EmailService) {}
  async place(order: Order) { /* domain logic */ }
}
```

```py
class OrderService:
    def __init__(self, order_repo: OrderRepository, email_service: EmailService):
        ...
```

```go
type OrderService struct{ repo OrderRepository; email Email }
func (s *OrderService) Place(o Order) error { ... }
```

### Typed Errors

```ts
class AppError extends Error {}
class NotFoundError extends AppError { constructor() { super("not found", 404); } }
class ValidationError extends AppError { constructor(msg) { super(msg, 400); } }
```

```py
class AppError(Exception):
    def __init__(self, message, code, status_code): ...
class NotFoundError(AppError): ...
class ValidationError(AppError): ...
```

### Configuration

```ts
function requiredEnv(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`Missing env: ${name}`);
  return v;
}
```

```py
from pydantic_settings import BaseSettings
class Settings(BaseSettings): ...
```

## When to Use This

- Starting a new project -> read `references/technology-selection.md` first
- Reviewing existing code -> check against `references/api-design.md` and `references/release-checklist.md`
- Adding auth -> `references/auth-flow.md`
- Adding real-time -> use the typed-error and service-layer patterns from above

## License

MIT. Vendored from https://github.com/MiniMax-AI/skills.
