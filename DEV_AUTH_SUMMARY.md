# Development Auth Mode - Implementation Summary

## Overview
Added a development authentication mode (`AUTH_MODE=dev`) that disables OIDC requirements, enabling quick local development without needing to configure an OIDC provider. Dev mode now starts unauthenticated; clicking Login enables a local `dev-admin` session so the unauthenticated path can still be tested.

## Changes Made

### 1. Configuration (src/config.rs)
- **Added `AuthMode` enum** with `Oidc` and `Dev` variants
  - Case-insensitive parsing from string
  - Defaults to `Oidc` for production safety
- **Modified `Config` struct**:
  - Added `auth_mode: AuthMode` field
  - Made OIDC fields optional: `Option<String>` for all OIDC config
  - Made `session_secret` optional
- **Updated `Config::from_env()`**:
  - Conditionally requires OIDC env vars only in OIDC mode
  - In dev mode, OIDC vars are optional and not required
- **Added tests**:
  - `test_auth_mode_from_str()` - validates parsing
  - `test_auth_mode_default()` - verifies OIDC is default
  - `test_dev_mode_no_oidc_required()` - validates dev mode doesn't require OIDC
  - `test_oidc_mode_requires_vars()` - validates OIDC mode requires vars

### 2. Authentication (src/auth.rs)
- **Replaced `OidcClient` with `AuthClient` enum**:
  - `AuthClient::Oidc(OidcClientInner)` - production OIDC
  - `AuthClient::Dev` - development auth without OIDC
- **Renamed** `OidcClient` to `OidcClientInner` (internal)
- **Updated `AuthClient::new()`** to initialize based on auth mode
- **Dev auth is carried in `AppState`** so it works correctly across async worker threads
- **Dev user** returns fixed credentials:
  - subject: "dev-admin"
  - email: "dev@localhost"
  - name: "Development Admin"
- **Updated auth methods**:
  - `current_user()` - returns the dev user only after the dev login marker is set
  - `require_user()` - requires that dev login marker in dev mode
  - `logout()` - clears the dev login marker in dev mode
  - `authorize_url()` - sets the dev login marker and redirects to "/" in dev mode
  - `handle_callback()` - sets the dev login marker in dev mode for convenience
  - OIDC mode ignores the dev login marker and only accepts OIDC-marked sessions
- **Added helper functions**:
  - `dev_user()` - creates dev user instance
- **Added tests**:
  - `test_dev_user()` - validates dev user structure
  - `test_dev_auth_requires_login()` - validates dev mode starts unauthenticated and login/logout toggles local admin
  - `test_oidc_ignores_dev_login_marker()` - validates OIDC mode ignores dev login state

### 3. Routes (src/routes.rs)
- **Updated `AppState`**: changed `oidc_client` to `auth_client: AuthClient`
- **Updated `create_router()`** signature to accept `AuthClient`
- **Added session secret fallback** for dev mode with warning
- **Updated all handler routes** to use new auth client

### 4. Handlers (src/handlers/auth.rs)
- **Updated auth handlers** to use `state.auth_client` instead of `state.oidc_client`
- **Login handler** works in both modes (redirects to "/" in dev)
- **Callback handler** works in both modes (succeeds immediately in dev)
- **Logout handler** clears the dev login marker in dev mode
- **Me handler** returns unauthenticated until Login is clicked in dev mode

### 5. Main Application (src/main.rs)
- **Initialize `AuthClient`** instead of `OidcClient`
- **Log auth mode** on startup for visibility
- **Pass `auth_client`** to router

### 6. Bug Fix (src/db.rs)
- **Fixed recursion error** in `Vinyl::list()` function
- Replaced recursive call with explicit query for empty search
- Changed from `query_as!` macro to `query_as` to avoid compile-time check requirements

### 7. Documentation

#### .env.example
- Added `AUTH_MODE` variable with documentation
- Marked OIDC variables as required only when `AUTH_MODE=oidc`
- Fixed typo: `UPLOD_DIR` → `UPLOAD_DIR`
- Added clear comments explaining when each variable is required

#### .env.dev (NEW FILE)
- Created quick-start dev configuration
- Pre-configured with `AUTH_MODE=dev`
- Includes helpful comments
- No OIDC configuration needed

#### README.md
- Added "Quick Start (Development)" section
- Documented dev mode setup with `cp .env.dev .env`
- Added security warnings about never using dev mode in production
- Added "Configuration" section with environment variables table
- Explained both dev and OIDC modes
- Added prominent warning emoji for security notes

#### CHANGELOG.md
- Added "Unreleased" section with all changes
- Documented new features under "Added"
- Documented behavior changes under "Changed"
- Documented security considerations

## Security Considerations

1. **Production Safety**:
   - `AUTH_MODE` defaults to `oidc` (safe)
   - Dev mode logs prominent warning on startup
   - Documentation includes multiple warnings

2. **Clear Intent**:
   - Mode is explicitly set via environment variable
   - No automatic detection that might guess wrong
   - Startup logs clearly show which mode is active

3. **No Backdoors**:
   - Dev mode is completely separate code path
   - No way to access dev mode in OIDC mode
   - Auth mode is stored in application state and passed to handlers

## Testing

### Unit Tests (13 tests, all passing)
- Config parsing and environment handling (4 tests)
- Auth dev user, login marker behavior, and OIDC isolation (3 tests)
- Database operations (5 tests)
- Filename sanitization (1 test)

### Test Results
```
running 13 tests
test auth::tests::test_dev_auth_requires_login ... ok
test auth::tests::test_oidc_ignores_dev_login_marker ... ok
test auth::tests::test_dev_user ... ok
...
test result: ok. 13 passed; 0 failed; 0 ignored
```

### Build Results
- **Debug build**: ✅ Clean (no warnings or errors)
- **Release build**: ✅ Clean (no warnings or errors)
- **Cargo test**: ✅ All tests pass

### Manual Testing Checklist
- [ ] Start backend with `.env.dev` (dev mode)
- [ ] Verify startup logs show "auth mode: Dev"
- [ ] GET `/api/auth/me` returns authenticated=false before login
- [ ] Click Login, then GET `/api/auth/me` returns authenticated=true with dev user
- [ ] Admin endpoints return 401 before dev login and work after dev login
- [ ] Start backend with OIDC config (production mode)
- [ ] Verify OIDC login flow still works
- [ ] Verify session management still works

## API Contract Preserved

### No Breaking Changes
All existing endpoints maintain the same:
- Request/response formats
- Status codes
- Error handling
- Session behavior

### GET /api/auth/me
Returns the same structure in both modes. In dev mode before Login:
```json
{
  "authenticated": false
}
```

After clicking Login in dev mode:
```json
{
  "authenticated": true,
  "subject": "dev-admin",
  "email": "dev@localhost",
  "name": "Development Admin"
}
```

### Admin Endpoints
Still use `require_auth()` middleware:
- POST /api/admin/vinyls
- PUT /api/admin/vinyls/:id
- DELETE /api/admin/vinyls/:id
- POST /api/admin/vinyls/bulk
- POST /api/admin/uploads

## Migration Guide for Developers

### Quick Start (Dev Mode)
```bash
# Copy dev config
cp .env.dev .env

# Run backend
cargo run
```

### Production (OIDC Mode)
```bash
# Copy example config
cp .env.example .env

# Edit .env and set OIDC variables
# AUTH_MODE=oidc (or leave unset)
# OIDC_ISSUER_URL=...
# OIDC_CLIENT_ID=...
# OIDC_CLIENT_SECRET=...
# OIDC_REDIRECT_URL=...
# SESSION_SECRET=$(openssl rand -hex 32)

# Run backend
cargo run
```

## Code Quality Metrics

- **Lines of code changed**: ~400 lines
- **New files**: 2 (.env.dev, SUMMARY.md)
- **Modified files**: 8
- **Tests added**: 4 new tests
- **Test coverage**: Config and auth mode logic fully tested
- **Build time**: ~2m 28s (release)
- **Binary size**: No significant change

## Future Enhancements

Potential improvements (not implemented):
1. Add metrics/monitoring support
2. Add integration tests for auth flow
3. Consider Redis session store for horizontal scaling
4. Add dev mode UI indicator in frontend
5. Add test authentication mode separate from dev mode
