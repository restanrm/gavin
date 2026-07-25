# Gavin Vinyl Library

A Rust + React web application for browsing and managing a family vinyl record library.

## Features

- **Public Catalog**: Browse vinyl records in a responsive grid layout
- **Search**: Real-time search with debouncing for artist and title
- **Authentication**: OIDC-based login integration
- **Admin Controls**: 
  - Add individual vinyl records
  - Upload cover images
  - Bulk import via CSV
  - Delete records
- **Responsive Design**: Works seamlessly on mobile, tablet, and desktop
- **Accessible**: WCAG 2.1 AA compliant with keyboard navigation support
- **Dark Mode**: Light, dark, and system theme options with saved preferences
- **Installable PWA**: Add to a mobile home screen with offline shell caching

## Prerequisites

- Node.js 18+ 
- npm or yarn
- Backend API running (defaults to `http://localhost:3000`)

## Quick Start (Development)

### Full Local Dev Stack with mise

```bash
# The first run may ask you to trust this repository's mise.toml:
#   mise trust
mise run dev
```

This starts both required local components:
- Rust backend on `http://127.0.0.1:3000` with `AUTH_MODE=dev`
- Vite frontend on `http://127.0.0.1:5173`

### Backend Development Mode (No OIDC Setup Required)

For backend-only local development without setting up OIDC:

```bash
mise run dev:backend
# or manually:
cp .env.dev .env
cargo run
```

The backend will start in **development auth mode** (`AUTH_MODE=dev`) where:
- No OIDC configuration is required
- You start unauthenticated, so public/unauthenticated paths can be tested locally
- Clicking **Login** enables a local `dev-admin` session cookie for that browser
- Clicking **Logout** clears the local dev-admin state again
- Dev login state is ignored when `AUTH_MODE=oidc`, so the dev cookie cannot authenticate production/OIDC mode

⚠️ **Never use `AUTH_MODE=dev` in production!** It is intended only for local development.

### Backend Production Mode (OIDC)

For production or to test with real OIDC:

```bash
# Copy the example environment config
cp .env.example .env

# Edit .env and set:
# - AUTH_MODE=oidc (or leave unset, oidc is default)
# - OIDC_ISSUER_URL, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET
# - OIDC_REDIRECT_URL (e.g., http://localhost:3000/api/auth/callback)
# - SESSION_SECRET (generate with: openssl rand -hex 32)

# Build and run
cargo build
cargo run
```

### k3d Dev Deployment

To validate the Helm stack locally with Podman + k3d:

```bash
mise run k3d:dev
kubectl port-forward -n gavin-dev svc/gavin 8080:80
```

Then open `http://127.0.0.1:8080`.

## Deployment

For production deployment with Podman and Kubernetes, see:
- **[Deployment Guide](docs/deployment.md)** - Complete guide for Podman containers, Kubernetes/Helm, and ArgoCD
- **[Helm Chart](charts/gavin/README.md)** - Kubernetes deployment via Helm

Quick Podman deployment:
```bash
podman build -t gavin:latest .
podman run -d -p 3000:3000 \
  -e OIDC_ISSUER_URL=https://your-oidc-provider.com \
  -e OIDC_CLIENT_ID=your-client-id \
  -e OIDC_CLIENT_SECRET=your-secret \
  -e OIDC_REDIRECT_URL=http://localhost:3000/api/auth/callback \
  -e SESSION_SECRET=$(openssl rand -hex 32) \
  gavin:latest
```

## Getting Started

### Installation

```bash
# Install dependencies
npm install
```

### Development

```bash
# Start development server (with API proxy to localhost:3000)
npm run dev
```

The app will be available at `http://localhost:5173`

### Building for Production

```bash
# Type check
npm run typecheck

# Build
npm run build

# Preview production build
npm run preview
```

The built files will be in the `dist/` directory.

## Testing

```bash
# Run tests
npm test

# Run tests with UI
npm test:ui

# Run tests in watch mode
npm test -- --watch
```

## Backend API Integration

The frontend expects the following API endpoints:

### Public Endpoints

- `GET /api/vinyls?search=` - Get all vinyls (with optional search)
- `GET /api/auth/me` - Get authentication status
- `GET /api/auth/login` - Redirect to OIDC login
- `POST /api/auth/logout` - Logout

### Admin Endpoints (require authentication)

- `POST /api/admin/vinyls` - Create vinyl record
- `PUT /api/admin/vinyls/:id` - Update vinyl record
- `DELETE /api/admin/vinyls/:id` - Delete vinyl record
- `GET /api/admin/albums/search?artist=` - Search MusicBrainz albums by artist for manual selection
- `POST /api/admin/vinyls/bulk` - Bulk import vinyls
- `POST /api/admin/vinyls/import-cover` - Import a vinyl by uploading an album-cover photo (multipart/form-data)
- `POST /api/admin/vinyls/import-cover-candidate` - Import a selected MusicBrainz candidate from cover-photo matching
- `POST /api/admin/uploads` - Upload cover image (multipart/form-data)

### Data Types

**Vinyl Object**:
```typescript
{
  id: string;
  artist: string;
  title: string;
  release_year?: number;
  notes?: string;
  cover_image_url?: string;
  created_at: string;
  metadata_status: 'pending' | 'complete' | 'needs_choice' | 'not_found' | 'error' | 'disabled';
  metadata_source?: string;
  metadata_source_id?: string;
  metadata_source_url?: string;
  metadata_candidates?: string; // JSON array of possible matches when metadata_status is needs_choice
  metadata_error?: string;
  metadata_checked_at?: string;
}
```

**Auth Response**:
```typescript
{
  authenticated: boolean;
  subject?: string;
  email?: string;
  name?: string;
}
```

## Project Structure

```
src/
├── components/         # React components
│   ├── VinylCard.tsx
│   ├── VinylCatalog.tsx
│   ├── SearchBar.tsx
│   ├── LoginButton.tsx
│   ├── AdminPanel.tsx
│   ├── VinylForm.tsx
│   ├── BulkImport.tsx
│   └── ImageUpload.tsx
├── hooks/             # Custom React hooks
│   ├── useAuth.ts
│   ├── useVinyls.ts
│   └── useDebounce.ts
├── utils/             # Utility functions
│   ├── api.ts        # API client
│   └── csv.ts        # CSV parsing
├── types/             # TypeScript types
│   └── index.ts
├── test/              # Test setup
│   └── setup.ts
├── App.tsx            # Main application
├── App.css            # Application styles
├── main.tsx           # Entry point
└── index.css          # Global styles
```

## CSV Bulk Import Format

The bulk import feature accepts CSV data in the following format:

```csv
artist,title,year,notes,cover_url
```

- **artist** (required): Artist name
- **title** (required): Album title
- **year** (optional): Release year (must be a number)
- **notes** (optional): Additional notes
- **cover_url** (optional): URL to cover image

Bulk imports are enriched against MusicBrainz one album at a time. If several plausible albums match one CSV row, the row is created with `metadata_status: "needs_choice"` and the candidate albums are stored in `metadata_candidates` for admin review.

### Example

```csv
The Beatles,Abbey Road,1969,Final studio album,https://example.com/abbey.jpg
Pink Floyd,The Dark Side of the Moon,1973
Miles Davis,Kind of Blue,1959,Essential jazz album
```

## Album Cover Photo Import

Admins can import a vinyl from a photo of the album cover. Gavin asks the configured vision provider to identify the cover image, resolves the detected album terms against MusicBrainz, and stores the clean official Cover Art Archive URL for the matched album instead of storing the uploaded photo.

MusicBrainz and Cover Art Archive are still used for free/open metadata and artwork. A freely available non-AI reverse-cover search API is not currently configured, so visual identification can use Gemini (`ALBUM_COVER_RECOGNITION_PROVIDER=gemini`) or ChatGPT/OpenAI (`ALBUM_COVER_RECOGNITION_PROVIDER=openai`). Because visual recognition can be wrong, Gavin always shows the uploaded cover next to candidate jackets and asks the admin to click the matching jacket before importing.

If imports fail with `429 Too Many Requests`, check the configured provider's free-tier/API limits. Gavin retries transient 429s, but quota exhaustion must be fixed in the provider account or by using another key/provider.

If Gemini returns a `404 Not Found` for the configured model, list the models available to your API key and choose one that supports `generateContent`:

```bash
curl "https://generativelanguage.googleapis.com/v1beta/models?key=$GEMINI_API_KEY"
```

Use the returned model name without or with the `models/` prefix, for example `GEMINI_ALBUM_COVER_MODEL=gemini-2.0-flash`.

## Mobile/PWA Support

The frontend includes a web app manifest, mobile icons, theme metadata, and a service worker. After serving the production build over HTTPS, mobile users can install Gavin from the browser menu ("Add to Home Screen" on iOS/Android). The service worker caches the application shell and static assets; API data still comes from the backend.

## Browser Support

- Chrome/Edge (last 2 versions)
- Firefox (last 2 versions)
- Safari (last 2 versions)
- Modern mobile browsers

## Accessibility Features

- Semantic HTML elements
- ARIA labels and roles where needed
- Keyboard navigation support
- Focus indicators
- Screen reader friendly
- Sufficient color contrast (WCAG AA)

## Configuration

## Configuration

### Environment Variables

Key configuration options:

- `AUTH_MODE` - Authentication mode: `oidc` (default, production) or `dev` (development)
- `DATABASE_URL` - SQLite database path (default: `sqlite://data/gavin.db`)
- `UPLOAD_DIR` - Directory for uploaded files (default: `data/uploads`)
- `PUBLIC_DOMAIN` - Public domain used for default callback URLs (default: `gavin.restanrm.fr`)
- `HOST` / `PORT` - Server bind address (default: `0.0.0.0:3000`)
- `ALBUM_METADATA_ENABLED` - Enable internet album metadata enrichment (default: `true`)
- `ALBUM_METADATA_USER_AGENT` - Optional MusicBrainz user agent; recommended for public deployments
- `ALBUM_COVER_RECOGNITION_PROVIDER` - Album-cover visual recognition provider: `gemini`, `openai`, or `disabled` (defaults to `gemini` when `GEMINI_API_KEY` is set, otherwise `openai`)
- `GEMINI_API_KEY` - Gemini API key for album-cover recognition (Google AI Studio free tier)
- `GEMINI_BASE_URL` - Gemini API base URL (default: `https://generativelanguage.googleapis.com`)
- `GEMINI_ALBUM_COVER_MODEL` - Gemini vision model used for album-cover recognition (default: `gemini-2.0-flash`)
- `OPENAI_API_KEY` - OpenAI API key required when using `ALBUM_COVER_RECOGNITION_PROVIDER=openai`
- `OPENAI_BASE_URL` - OpenAI-compatible API base URL (default: `https://api.openai.com`)
- `OPENAI_ALBUM_COVER_MODEL` - ChatGPT vision model used for album-cover recognition (default: `gpt-4o-mini`)
- `MUSICBRAINZ_BASE_URL` - MusicBrainz API base URL (default: `https://musicbrainz.org`)
- `COVER_ART_ARCHIVE_BASE_URL` - Cover Art Archive API base URL (default: `https://coverartarchive.org`)

**Album Metadata Enrichment**:
- Creating a vinyl (single or bulk) performs a best-effort lookup via MusicBrainz and stores release year, cover art URL, source URL, and lookup status in SQLite.
- Cover-photo imports use the configured visual recognition provider to find MusicBrainz candidates, then store the official Cover Art Archive image URL for the imported album.
- If multiple plausible album matches exist, Gavin marks the record as `needs_choice` and stores candidate choices instead of guessing.
- On startup, Gavin launches an asynchronous background check that retries rows with pending, failed, or missing metadata lookups.

**OIDC Configuration** (required when `AUTH_MODE=oidc`):
- `OIDC_ISSUER_URL` - OIDC provider issuer URL
- `OIDC_CLIENT_ID` - OAuth client ID
- `OIDC_CLIENT_SECRET` - OAuth client secret
- `OIDC_REDIRECT_URL` - OAuth callback URL (defaults to `https://${PUBLIC_DOMAIN}/api/auth/callback`)
- `OIDC_REDIRECT_URL` - Callback URL (e.g., `http://localhost:3000/api/auth/callback`)
- `SESSION_SECRET` - Secret for session encryption (generate with `openssl rand -hex 32`)

**Development Mode** (`AUTH_MODE=dev`):
- Disables OIDC requirements for local development
- No OIDC or session secrets required
- Starts unauthenticated; clicking **Login** creates a local `dev-admin` session
- The dev login marker is ignored in `AUTH_MODE=oidc`
- Suitable for local development and testing

See `.env.example` for full configuration or `.env.dev` for quick development setup.

### Changing API URL

In production, you may need to configure the API base URL. The proxy in `vite.config.ts` is only for development.

For production, you can:

1. Use the same domain (recommended)
2. Set up a reverse proxy (nginx, etc.)
3. Configure CORS on your backend

### Vite Configuration

Edit `vite.config.ts` to change:
- API proxy settings
- Build options
- Port numbers

## Troubleshooting

### API Connection Issues

If the frontend can't connect to the API:

1. Verify backend is running on port 3000
2. Check browser console for errors
3. Verify proxy configuration in `vite.config.ts`

### Build Issues

```bash
# Clear node modules and reinstall
rm -rf node_modules package-lock.json
npm install

# Clear Vite cache
rm -rf node_modules/.vite
```

## Contributing

1. Follow the existing code style (TypeScript strict mode)
2. Add tests for new features
3. Ensure accessibility standards are met
4. Update documentation as needed

## License

[Your License Here]
