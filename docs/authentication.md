# Authentication design

xui-rs supports the two authentication mechanisms implemented by 3x-ui
v3.6.0. They deliberately have different lifecycles.

## API tokens

Bearer tokens are the default recommendation for programmatic clients. 3x-ui
accepts them on every `/panel/api/*` endpoint and skips CSRF validation after
the token has been verified. The token is held in `SecretString`, omitted from
`Debug`, and never written to tracing fields.

## Cookie sessions

Cookie authentication follows the same sequence as the 3x-ui web client:

1. `GET /csrf-token` creates a pre-auth session and returns its CSRF token.
2. `POST /login` sends that token in `X-CSRF-Token`; the cookie jar sends the
   matching signed `3x-ui` cookie automatically.
3. Successful login updates the same session cookie. Unsafe session requests
   continue to send the CSRF token.
4. `POST /logout` clears the server-side session and the local cached CSRF
   token.

The SDK does not parse `Max-Age`, copy a complete `Set-Cookie` value into the
`Cookie` request header, retain passwords, or perform hidden automatic login.
Those behaviours are both unnecessary with a cookie jar and surprising for
applications that control credential access.

CSRF initialization is synchronized across cloned clients. If an unsafe
session request receives HTTP 403, the client discards its cached CSRF value,
creates a fresh pre-auth session, and retries once. Retrying is safe here
because the 3x-ui CSRF middleware rejects the original request before its
handler executes.

## Upstream references

This design is based on the v3.6.0 implementations in:

- `internal/web/controller/index.go`
- `internal/web/controller/api.go`
- `internal/web/middleware/security.go`
- `internal/web/session/csrf.go`
- `internal/web/session/session.go`
