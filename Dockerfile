# syntax=docker/dockerfile:1

# ============================================================================
# Stage 1: Build Frontend
# ============================================================================
FROM node:20-bookworm-slim AS frontend-builder

WORKDIR /build

# Copy frontend dependencies
COPY package.json package-lock.json ./

# Install dependencies
RUN npm ci --prefer-offline --no-audit

# Copy frontend source
COPY index.html tsconfig.json tsconfig.node.json vite.config.ts ./
COPY public/ ./public/
COPY src/*.tsx src/*.css src/*.ts ./src/
COPY src/components/ ./src/components/
COPY src/hooks/ ./src/hooks/
COPY src/utils/ ./src/utils/
COPY src/types/ ./src/types/
COPY src/test/ ./src/test/

# Build frontend
RUN npm run build

# ============================================================================
# Stage 2: Build Rust Backend
# ============================================================================
FROM rust:1.88-bookworm AS backend-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy Cargo manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src/ ./src/
COPY migrations/ ./migrations/

# Touch main.rs to force rebuild with actual code
RUN touch src/main.rs && \
    cargo build --release

# ============================================================================
# Stage 3: Runtime Image
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -g 1000 gavin && \
    useradd -r -u 1000 -g gavin -s /sbin/nologin -c "Gavin application user" gavin

# Create application directories
RUN mkdir -p /app/dist /app/data/uploads /app/migrations && \
    chown -R gavin:gavin /app

WORKDIR /app

# Copy binary from builder
COPY --from=backend-builder /build/target/release/gavin /app/gavin

# Copy frontend dist from builder
COPY --from=frontend-builder /build/dist /app/dist

# Copy migrations
COPY migrations/ /app/migrations/

# Change ownership to non-root user
RUN chown -R gavin:gavin /app

# Switch to non-root user
USER gavin

# Set production environment variables
ENV HOST=0.0.0.0 \
    PORT=3000 \
    FRONTEND_DIR=/app/dist \
    UPLOAD_DIR=/app/data/uploads \
    DATABASE_URL=sqlite:///app/data/gavin.db \
    PUBLIC_DOMAIN=gavin.restanrm.fr \
    RUST_LOG=info,gavin=info

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/bin/sh", "-c", "wget --no-verbose --tries=1 --spider http://localhost:3000/api/health || exit 1"]

# Run the application
CMD ["/app/gavin"]
