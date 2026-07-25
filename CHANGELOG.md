# Changelog

All notable changes to the Gavin Vinyl Library project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Admin metadata review highlighting** that visually flags albums with incomplete metadata in the catalog and edit dialog so admins can quickly spot records needing attention.
- **Clickable metadata candidate selection** in album edit mode, letting admins apply a reviewed MusicBrainz match to an existing vinyl instead of only opening the source information link.

## [0.2.1] - 2026-07-25

### Added
- **Automated GitHub release workflow** that prepares version files, validates builds/tests/Helm chart, tags releases, publishes GHCR container images and Helm charts, and creates GitHub releases from changelog notes.
- **Local release preparation helpers** via `scripts/prepare-release.sh`, `scripts/extract-release-notes.sh`, and `mise run release:prepare` / `mise run release:notes`.
- **Internet album metadata enrichment** using MusicBrainz and Cover Art Archive when admins add vinyls individually or in bulk.
- **Metadata choice marking** for bulk-imported albums with multiple plausible MusicBrainz matches (`metadata_status=needs_choice`).
- **Async startup metadata completeness job** that retries pending, failed, or missing metadata lookups after boot.
- **mise dev tasks** for local backend/frontend startup, tests, Podman builds, and k3d dev deployment.
- **PWA/mobile support** with manifest, service worker, mobile metadata, and generated app icons/logo.
- **Configurable public domain** via `PUBLIC_DOMAIN` and Helm `domain`, defaulting to `gavin.restanrm.fr`.
- **Development Authentication Mode**: New `AUTH_MODE=dev` environment variable option for simplified local development
  - Bypasses OIDC authentication requirements
  - Starts unauthenticated so public/unauthenticated paths can be tested locally
  - Clicking login creates a local `dev-admin` session; logout clears it
  - Dev login state is ignored in OIDC mode
  - No need to configure OIDC provider for local testing
  - Session secret optional in dev mode
  - Includes `.env.dev` example file for quick setup
  - Documentation updated with quick start guide

### Changed
- OIDC configuration is now optional when `AUTH_MODE=dev` (required when `AUTH_MODE=oidc` or unset)
- `SESSION_SECRET` is optional in dev mode (required for OIDC mode)
- `AuthClient` enum introduced to support both OIDC and dev authentication modes
- Backend now logs authentication mode on startup for clarity
- Helm defaults now pull `ghcr.io/restanrm/gavin:<appVersion>`, currently `ghcr.io/restanrm/gavin:v0.2.0`.
- Production Helm values now keep a single replica by default because SQLite is the intended database for this app.
- Container documentation now prefers Podman while keeping OCI/Docker compatibility notes.

### Security
- Added prominent warnings about never using dev mode in production
- Dev mode logs warning on startup to prevent accidental production use
- Dev login markers are scoped to `AUTH_MODE=dev` and are not honored in OIDC mode

## [0.1.0] - 2026-07-25

### Added

#### Application Features
- Initial MVP release of Gavin Vinyl Library
- REST API backend built with Rust and Axum
- React/TypeScript frontend with Vite
- OIDC authentication via Pocket ID or compatible providers
- SQLite database for vinyl catalog storage
- Public vinyl catalog browsing with search
- Admin panel for authenticated users:
  - Add/edit/delete vinyl records
  - Upload cover images
  - Bulk import via CSV
- Responsive design with dark mode support
- Health check endpoint at `/api/health`

#### Deployment Assets
- **Docker Support:**
  - Multi-stage production Dockerfile
  - Frontend build stage (Node.js 20)
  - Backend build stage (Rust 1.75)
  - Minimal runtime image (Debian Bookworm Slim)
  - Non-root user execution (UID/GID 1000)
  - Health check configuration
  - `.dockerignore` for efficient builds
  
- **Kubernetes/Helm Chart:**
  - Complete Helm chart under `charts/gavin/`
  - Production-ready Kubernetes manifests
  - Deployment with configurable replicas
  - Service (ClusterIP) and optional Ingress
  - PersistentVolumeClaim for data persistence
  - ConfigMap for application configuration
  - Secret management (inline or external)
  - Liveness and readiness probes
  - Security contexts (non-root, seccomp, capabilities dropped)
  - Resource limits and requests
  - Optional HorizontalPodAutoscaler
  - Optional PodDisruptionBudget
  - Optional NetworkPolicy
  - ServiceAccount with RBAC
  - ArgoCD-friendly structure

- **Documentation:**
  - Comprehensive deployment guide (`docs/deployment.md`)
  - Docker deployment instructions
  - Kubernetes/Helm deployment guide
  - ArgoCD integration examples
  - OIDC configuration guide for Pocket ID
  - Troubleshooting section
  - Backup and recovery procedures
  - Security best practices
  - Helm chart README with all parameters
  - Example values files for dev and production

#### Configuration
- Environment-based configuration
- Sensible defaults for all non-secret values
- Support for existing secrets (recommended for production)
- Configurable persistence options
- TLS/SSL support via Ingress
- Health check endpoints

### Infrastructure
- Multi-stage Docker builds for optimal image size
- Layer caching for faster builds
- Database migrations included in image
- Persistent volume support for SQLite database and uploads
- Horizontal scaling support (with caveats for SQLite)
- Zero-downtime deployment support via rolling updates
- Graceful shutdown handling

### Security
- Non-root container execution
- Read-only root filesystem capable
- Security contexts with dropped capabilities
- Seccomp profile support
- TLS/HTTPS support via Ingress
- Secure session cookies (configurable)
- Secret management via Kubernetes Secrets
- Support for external secret management (Sealed Secrets, External Secrets Operator)
- No secrets in container images

### Operational Excellence
- Health check endpoints for monitoring
- Structured logging support (JSON)
- Configurable log levels via RUST_LOG
- Resource limits and requests defined
- Probe configurations for liveness and readiness
- PodDisruptionBudget for high availability
- HPA for automatic scaling
- Anti-affinity rules for pod distribution

### Developer Experience
- Development values file for local testing
- Port-forward support for local access
- Helm lint and template validation
- Clear NOTES.txt with post-install instructions
- Comprehensive troubleshooting guide

### Known Limitations
- SQLite database limits horizontal scaling (single writer)
- ReadWriteOnce PVC requires pod affinity for multi-replica
- No built-in monitoring/metrics endpoint (future enhancement)
- Session storage in database (consider Redis for scale)

### Deployment Notes
- Minimum Kubernetes version: 1.20+
- Minimum Helm version: 3.8+
- Requires OIDC provider configuration
- Requires persistent storage provisioner if persistence enabled
- Callback URL must match OIDC provider configuration exactly

### Migration Path
This is the initial release. No migrations required.

### Breaking Changes
None (initial release)

---

## Release Checklist Template

Preferred release path: run the manual GitHub Actions workflow `.github/workflows/release.yml` from the default branch with the target semantic version.

The workflow:

- [ ] Updates version files (`Cargo.toml`, `Cargo.lock`, `package.json`, `package-lock.json`, `charts/gavin/Chart.yaml`)
- [ ] Promotes `CHANGELOG.md` `Unreleased` notes to the new version
- [ ] Runs backend/frontend tests and builds
- [ ] Runs `helm lint charts/gavin` and `helm template`
- [ ] Builds the container image before publishing
- [ ] Creates a release commit and annotated `vX.Y.Z` tag
- [ ] Pushes GHCR image tags (`X.Y.Z`, `vX.Y.Z`, and optionally `latest`)
- [ ] Creates the GitHub release with notes from `CHANGELOG.md`

For local preparation only, use `VERSION=0.2.0 mise run release:prepare`.

[Unreleased]: https://github.com/restanrm/gavin/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/restanrm/gavin/compare/v0.1.0...v0.2.1
[0.1.0]: https://github.com/restanrm/gavin/releases/tag/v0.1.0
