# Development Notes

## Project Structure

```
gavin/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration loading
│   ├── db.rs                # Database models and queries
│   ├── error.rs             # Error types
│   ├── auth.rs              # OIDC authentication
│   ├── routes.rs            # Route setup
│   └── handlers/
│       ├── mod.rs           # Handler modules
│       ├── public.rs        # Public endpoints
│       ├── auth.rs          # Auth endpoints
│       └── admin.rs         # Admin endpoints
├── migrations/
│   └── 20260725000001_create_vinyls.sql
├── tests/
│   └── api_tests.rs         # Integration test stubs
├── Cargo.toml
├── README.md                # User documentation
└── .env.example             # Environment template
```

## Testing Strategy

### Unit Tests (Implemented)
- `src/db.rs`: Database operations including:
  - Create and retrieve vinyls
  - Case-insensitive search
  - Update and delete operations
  - Proper sorting
- `src/handlers/admin.rs`: Filename sanitization

### Integration Tests (Stubbed)
- `tests/api_tests.rs`: Placeholder for full API tests
- Requires: test database setup, mock OIDC provider

## Authentication Flow

1. User visits `/api/auth/login`
2. Redirected to OIDC provider with state/nonce
3. User authenticates with provider
4. Provider redirects to `/api/auth/callback?code=...&state=...`
5. Backend verifies state, exchanges code for ID token
6. ID token validated, user info stored in session
7. Session cookie set (SQLite-backed)
8. All `/api/admin/*` endpoints require valid session

## Database

- SQLite with automatic migrations on startup
- Schema in `migrations/20260725000001_create_vinyls.sql`
- Connection pool size: 5
- Session store shares same database

## Search Implementation

Search is implemented with SQLite `LIKE` operator:
- Case-insensitive via `LOWER()` function
- Matches artist OR title
- Whitespace trimmed from query
- `%` wildcards for substring matching

Example:
```sql
SELECT * FROM vinyls
WHERE LOWER(artist) LIKE '%beatles%'
   OR LOWER(title) LIKE '%beatles%'
ORDER BY LOWER(artist), LOWER(title)
```

## File Uploads

- Stored in `UPLOAD_DIR` (default: `data/uploads`)
- Filename sanitization prevents directory traversal
- Timestamped to avoid collisions
- Served under `/uploads` path
- No file type restrictions (add if needed)

## Session Management

- Tower-sessions with SQLite backend
- 24-hour inactivity timeout
- Secure cookies (configurable)
- No signing (consider adding for production)

## Known Limitations

1. **No role-based access**: All OIDC users are admins
2. **No file type validation**: Uploads accept any file
3. **No file size limits**: Could cause issues with large uploads
4. **SQLite concurrency**: Limited concurrent writes
5. **No pagination**: List endpoint returns all records
6. **Session signing disabled**: Sessions not cryptographically signed

## Future Improvements

1. Add role-based access control (RBAC)
2. Implement pagination for vinyl list
3. Add image optimization for cover uploads
4. File type and size validation
5. Rate limiting on uploads and admin endpoints
6. Batch operations in transactions for bulk create
7. Full integration test suite with test fixtures
8. PostgreSQL support for production deployments
9. GraphQL API option
10. Real-time updates via WebSocket

## Performance Considerations

- Indexes on `artist`, `title`, and `created_at`
- Connection pooling (5 connections)
- Async I/O throughout
- Static file serving via tower-http
- No N+1 queries (single query for list/search)

## Security Considerations

- OIDC provider handles authentication
- Session cookies (configure secure flag for HTTPS)
- SQL injection prevented by SQLx parameter binding
- Directory traversal prevented in file uploads
- CORS enabled (configure for production)
- All admin endpoints require authentication

## Environment Variables Reference

See `.env.example` for full list. Critical ones:
- `DATABASE_URL`: SQLite file path
- `OIDC_*`: Provider configuration
- `SESSION_SECRET`: Must be random and constant
- `COOKIE_SECURE`: true for HTTPS, false for HTTP

## Building and Running

```bash
# Development
cargo run

# With logs
RUST_LOG=debug cargo run

# Production build
cargo build --release

# Run tests
cargo test

# Format code (if rustfmt installed)
cargo fmt

# Lint (if clippy installed)
cargo clippy
```

## Troubleshooting

### Database Locked
- Reduce concurrent operations
- Check connection pool size
- Consider PostgreSQL for high concurrency

### OIDC Errors
- Verify issuer URL ends with `/` if needed
- Check redirect URL matches provider config
- Ensure scopes (openid, email, profile) are enabled
- Check provider logs for authorization errors

### Session Issues
- Verify `SESSION_SECRET` is set and constant
- Check `COOKIE_SECURE` matches your protocol
- Ensure cookies are enabled in browser
- Check session table in database

### Build Errors
- Rust 1.75+ required
- Run `cargo clean` and rebuild
- Check for conflicting dependency versions
