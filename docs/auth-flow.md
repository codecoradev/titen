# Authentication & Authorization Flow

This document describes how authentication and authorization work in Titen. There are two distinct concepts that are often confused — understanding the difference is critical.

---

## Two Auth Concepts

| # | Concept | Purpose | Mechanism |
|---|---|---|---|
| 1 | **Admin dashboard access** | Authenticate the operator to the Titen API and web dashboard | `TITEN_API_KEY` shared secret → cookie session |
| 2 | **Threads account OAuth** | Connect a Threads account so the scheduler can publish on its behalf | OAuth 2.0 authorization code flow |

These are **completely independent**. The admin API key has nothing to do with Threads OAuth tokens, and vice versa.

---

## Admin Login Flow

```
┌────────────┐
│   User     │  Enters API key at /login form
└─────┬──────┘
      │
      ▼
POST /api/auth/login
      │  Header: X-API-Key: <titen_api_key>
      │
      ▼
Backend validates API key against TITEN_API_KEY env var
      │
      │  match → generate session
      ▼
Set-Cookie: titen_session=<session_value>
      │  HttpOnly; SameSite=Strict; Max-Age=604800; Path=/
      │
      ▼
Subsequent requests: browser auto-attaches cookie
      │
      ▼
GET /api/auth/session → validates session → returns user info
      │
      ▼
POST /api/auth/logout → clears titen_session cookie
```

### Step-by-step

1. **User enters the API key** at the `/login` page in the SvelteKit frontend.
2. **Frontend sends** `POST /api/auth/login` with the `X-API-Key` header containing the user-supplied key.
3. **Backend compares** the supplied key against the server's `TITEN_API_KEY` environment variable.
4. **On match**, the backend generates a session and sets the `titen_session` cookie.
5. **Subsequent requests** from the browser automatically include the cookie (standard browser behavior for same-origin requests).
6. **Session validation** — `GET /api/auth/session` checks the cookie and returns session status.
7. **Logout** — `POST /api/auth/logout` clears the cookie.

---

## Auth Middleware Priority Chain

The `api_key_auth` middleware checks for credentials in the following order. The first match wins.

```
Request arrives
    │
    ▼
1. X-API-Key header present? ──► validate against TITEN_API_KEY
    │ no
    ▼
2. ?api_key= query param?     ──► validate against TITEN_API_KEY
    │ no
    ▼
3. titen_session cookie?      ──► validate session
    │ no
    ▼
401 Unauthorized
```

| Priority | Source | Use case |
|---|---|---|
| 1 | `X-API-Key` header | API clients, CLI tools, MCP server |
| 2 | `api_key` query param | Webhooks, simple integrations where headers aren't available |
| 3 | `titen_session` cookie | Browser dashboard sessions |

---

## Cookie Security

The `titen_session` cookie is configured with strict security defaults:

| Attribute | Value | Rationale |
|---|---|---|
| `HttpOnly` | `true` | Prevents JavaScript (`document.cookie`) from reading the cookie — mitigates XSS token theft |
| `Secure` | `true` *(production)* | Cookie only sent over HTTPS. **Must** be enabled in production behind a TLS reverse proxy. Omitted in dev (HTTP localhost). |
| `SameSite` | `Strict` | Cookie is never sent on cross-site requests — mitigates CSRF |
| `Max-Age` | `604800` (7 days) | Session expires after one week; user must re-authenticate |
| `Path` | `/` | Cookie is sent on all routes |

> **Note:** The API key is **never** stored in `localStorage` or any client-accessible storage. After login, only the opaque session cookie is used. This eliminates the exposure surface entirely.

---

## Threads OAuth Flow

OAuth is used to connect Threads accounts so the scheduler can publish posts on their behalf. This is **not** used for admin authentication.

```
User clicks "Connect Account" in dashboard
    │
    ▼
Redirect to Threads authorize URL
    │  https://threads.net/oauth/authorize
    │    ?client_id=...
    │    &redirect_uri=.../auth/callback
    │    &scope=threads_basic,threads_content_publish
    │    &response_type=code
    │
    ▼
User logs into Threads and authorizes Titen
    │
    ▼
Threads redirects back to:
    │  /auth/callback?code=<authorization_code>
    │
    ▼
Backend exchanges code for access token
    │  POST https://graph.threads.net/oauth/access_token
    │    client_id, client_secret, code, grant_type=authorization_code
    │
    ▼
Store token in accounts table (encrypted at rest)
    │
    ▼
Scheduler uses stored token to publish on behalf of this account
```

---

## Token Management

| Concern | Implementation |
|---|---|
| **Storage** | `access_token` stored in the `accounts` table, encrypted at rest |
| **Refresh flow** | When a token is near expiry, the scheduler refreshes it using the Threads refresh endpoint before publishing |
| **Expiry checking** | The scheduler checks token validity before each publish attempt; expired tokens trigger a refresh or are marked for re-auth |
| **Batch check** | `POST /api/auth/check_tokens` endpoint validates all stored tokens in a single batch call — useful for diagnostics and dashboard status display |

### Token lifecycle

```
Token stored (encrypted)
    │
    ▼
Scheduler tick → check token expiry
    │
    ├── valid ──► proceed with publish
    │
    └── expiring/expired
            │
            ▼
        Refresh token via Threads API
            │
            ├── success ──► update stored token → proceed with publish
            │
            └── failure ──► mark account as needs_reauth → skip schedule
```

---

## Dev Mode

When the `TITEN_API_KEY` environment variable is **not set**, Titen runs in **dev mode**:

- **All endpoints are open** — no authentication is required.
- The `api_key_auth` middleware passes through all requests without checking credentials.
- The `/login` page and session endpoints are effectively no-ops.

> **Warning:** Dev mode is intended for local development only. Never deploy to production without setting `TITEN_API_KEY`. If you accidentally deploy without it, **every endpoint is fully exposed** — including account creation, token management, and post publishing. Always verify `TITEN_API_KEY` is set in your production environment variables before starting the server.
>
> **Recommendation:** Add a startup check in production that refuses to boot if `TITEN_API_KEY` is unset and `TITEN_ENV=production`.

To enable authentication, simply set the environment variable:

```bash
export TITEN_API_KEY="your-secure-api-key-here"
```
