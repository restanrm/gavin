# Gavin Repository Agent Notes

These instructions apply only to this repository.

## Project context

Gavin is a family vinyl library application with:
- Rust/Axum backend in `src/*.rs` using SQLite via `sqlx`.
- React/TypeScript frontend in `src/components`, `src/hooks`, `src/utils`, and `src/types`.
- Album metadata enrichment in `src/album_metadata.rs` using MusicBrainz, Cover Art Archive, vision providers for cover recognition, and French retailer hints (FNAC/Cultura) as fallback signals.
- Admin vinyl APIs and upload/import handlers in `src/handlers/admin.rs`; public catalog/detail APIs in `src/handlers/public.rs`; route wiring in `src/routes.rs`.
- User-facing documentation in `README.md` and deployment docs under `docs/` and `charts/`.

## Change requirements

- Update `CHANGELOG.md` after **all changes**: code, behavior, documentation, configuration, tests, styles, or agent/project instructions.
- Put new entries under `## [Unreleased]` unless the user is explicitly preparing a release section.
- Use Keep a Changelog categories (`Added`, `Changed`, `Fixed`, `Removed`, `Security`) where appropriate.
- Update README/docs when behavior, APIs, configuration, or user workflows change.
- Do not create commits; the user creates commits.

## Validation

Run targeted checks for the files changed, and prefer full validation before finishing:
- Frontend typecheck: `npm run typecheck`
- Frontend tests: `npm test -- --run`
- Frontend lint: `npm run lint`
- Backend tests: `cargo test`
- Rust formatting: `cargo fmt --check` when `rustfmt` is available; if unavailable, say so explicitly.
