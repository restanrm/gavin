# Deployment Guide

This guide covers deployment options for the Gavin Vinyl Library application.

The application intentionally uses a simple SQLite database. Keep the default single application replica unless you later migrate to a multi-writer database such as PostgreSQL.

The default public domain is `gavin.restanrm.fr`. Override `PUBLIC_DOMAIN` or Helm `domain` for tests/staging.

## Table of Contents

- [Container Deployment (Podman)](#container-deployment-podman)
- [Kubernetes/Helm Deployment](#kuberneteshelm-deployment)
- [ArgoCD Deployment](#argocd-deployment)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)

## Container Deployment (Podman)

### CI Build

GitHub Actions builds the container image on pushes to `main`, pull requests, and manual `workflow_dispatch` runs using `.github/workflows/container.yml`. The workflow only validates the Docker build; it does not push images to a registry.

### Building the Image

Build the production-ready container image with Podman:

```bash
podman build -t gavin:latest .
```

For a specific version:

```bash
podman build -t gavin:0.1.0 .
```

Docker users can replace `podman` with `docker`; the image file is OCI-compatible.

### Running with Podman

#### Using Podman Run

```bash
podman run -d \
  --name gavin \
  -p 3000:3000 \
  -v gavin-data:/app/data \
  -e OIDC_ISSUER_URL=https://your-oidc-provider.com \
  -e OIDC_CLIENT_ID=your-client-id \
  -e OIDC_CLIENT_SECRET=your-client-secret \
  -e PUBLIC_DOMAIN=gavin.restanrm.fr \
  -e OIDC_REDIRECT_URL=https://gavin.restanrm.fr/api/auth/callback \
  -e SESSION_SECRET=$(openssl rand -hex 32) \
  gavin:latest
```

#### Using Compose

Create a Compose file (works with `podman-compose` or Docker Compose):

```yaml
version: '3.8'

services:
  gavin:
    image: gavin:latest
    build: .
    ports:
      - "3000:3000"
    environment:
      - HOST=0.0.0.0
      - PORT=3000
      - FRONTEND_DIR=/app/dist
      - UPLOAD_DIR=/app/data/uploads
      - DATABASE_URL=sqlite:///app/data/gavin.db
      - PUBLIC_DOMAIN=gavin.restanrm.fr
      - RUST_LOG=info,gavin=info
      - COOKIE_SECURE=true
      - OIDC_ISSUER_URL=${OIDC_ISSUER_URL}
      - OIDC_CLIENT_ID=${OIDC_CLIENT_ID}
      - OIDC_CLIENT_SECRET=${OIDC_CLIENT_SECRET}
      - OIDC_REDIRECT_URL=${OIDC_REDIRECT_URL}
      - SESSION_SECRET=${SESSION_SECRET}
    volumes:
      - gavin-data:/app/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:3000/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s

volumes:
  gavin-data:
```

Run with:

```bash
# Create .env file with secrets
cat > .env << EOF
OIDC_ISSUER_URL=https://your-oidc-provider.com
OIDC_CLIENT_ID=your-client-id
OIDC_CLIENT_SECRET=your-client-secret
OIDC_REDIRECT_URL=https://gavin.restanrm.fr/api/auth/callback
SESSION_SECRET=$(openssl rand -hex 32)
EOF

# Start
podman-compose up -d

# View logs
podman-compose logs -f

# Stop
podman-compose down
```

### Container Image Registry

Push to a container registry:

```bash
# Container registry / compatible registries
podman tag gavin:latest yourusername/gavin:latest
podman push yourusername/gavin:latest

# Private registry
podman tag gavin:latest registry.yourdomain.com/gavin:latest
podman push registry.yourdomain.com/gavin:latest
```

## Kubernetes/Helm Deployment

### Prerequisites

- Kubernetes cluster (1.20+)
- kubectl configured
- Helm 3.8+
- Container image pushed to accessible registry

### Quick Start

1. **Create namespace:**

```bash
kubectl create namespace gavin
```

2. **Create secrets (recommended approach):**

```bash
kubectl create secret generic gavin-secrets \
  --namespace gavin \
  --from-literal=OIDC_ISSUER_URL=https://your-oidc-provider.com \
  --from-literal=OIDC_CLIENT_ID=your-client-id \
  --from-literal=OIDC_CLIENT_SECRET=your-client-secret \
  --from-literal=OIDC_REDIRECT_URL=https://gavin.restanrm.fr/api/auth/callback \
  --from-literal=SESSION_SECRET=$(openssl rand -hex 32)
```

3. **Install with Helm:**

```bash
helm install gavin ./charts/gavin \
  --namespace gavin \
  --set image.repository=your-registry.io/gavin \
  --set image.tag=0.1.0 \
  --set existingSecret=gavin-secrets
```

### Production Deployment

For production, customize `values-prod.yaml`:

```bash
# Copy and edit production values
cp charts/gavin/values-prod.yaml my-values.yaml
nano my-values.yaml

# Install with custom values
helm install gavin ./charts/gavin \
  --namespace gavin \
  --values my-values.yaml \
  --set existingSecret=gavin-secrets
```

### Enable Ingress

```bash
helm upgrade gavin ./charts/gavin \
  --namespace gavin \
  --reuse-values \
  --set ingress.enabled=true \
  --set ingress.className=nginx \
  --set ingress.hosts[0].host=gavin.restanrm.fr \
  --set ingress.hosts[0].paths[0].path=/ \
  --set ingress.hosts[0].paths[0].pathType=Prefix
```

### Verify Deployment

```bash
# Check pods
kubectl get pods -n gavin

# Check services
kubectl get svc -n gavin

# Check ingress
kubectl get ingress -n gavin

# View logs
kubectl logs -n gavin -l app.kubernetes.io/name=gavin -f

# Check health
kubectl exec -n gavin deployment/gavin -- wget -qO- http://localhost:3000/api/health
```

### Upgrade

```bash
# Upgrade to new version
helm upgrade gavin ./charts/gavin \
  --namespace gavin \
  --values my-values.yaml \
  --set image.tag=0.2.0

# Rollback if needed
helm rollback gavin -n gavin
```

### Uninstall

```bash
# Remove Helm release (keeps PVC)
helm uninstall gavin -n gavin

# Delete PVC if needed
kubectl delete pvc gavin -n gavin

# Delete namespace
kubectl delete namespace gavin
```

## ArgoCD Deployment

### Application Manifest

Create `argocd-gavin.yaml`:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: gavin
  namespace: argocd
  finalizers:
    - resources-finalizer.argocd.argoproj.io
spec:
  project: default
  
  source:
    repoURL: https://github.com/restanrm/gavin
    targetRevision: main
    path: charts/gavin
    helm:
      valueFiles:
        - values-prod.yaml
      parameters:
        - name: image.tag
          value: "0.1.0"
        - name: existingSecret
          value: gavin-secrets
  
  destination:
    server: https://kubernetes.default.svc
    namespace: gavin
  
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
      allowEmpty: false
    syncOptions:
      - CreateNamespace=true
      - ApplyOutOfSyncOnly=true
    retry:
      limit: 5
      backoff:
        duration: 5s
        factor: 2
        maxDuration: 3m
  
  ignoreDifferences:
    - group: apps
      kind: Deployment
      jsonPointers:
        - /spec/replicas
```

Apply:

```bash
# Create secrets first
kubectl create namespace gavin
kubectl create secret generic gavin-secrets \
  --namespace gavin \
  --from-literal=OIDC_ISSUER_URL=https://your-oidc-provider.com \
  --from-literal=OIDC_CLIENT_ID=your-client-id \
  --from-literal=OIDC_CLIENT_SECRET=your-client-secret \
  --from-literal=OIDC_REDIRECT_URL=https://gavin.restanrm.fr/api/auth/callback \
  --from-literal=SESSION_SECRET=$(openssl rand -hex 32)

# Deploy with ArgoCD
kubectl apply -f argocd-gavin.yaml

# Watch sync status
argocd app get gavin
argocd app sync gavin
```

### Using Sealed Secrets

For GitOps, use Sealed Secrets:

```bash
# Install sealed-secrets controller
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/controller.yaml

# Create and seal secret
kubectl create secret generic gavin-secrets \
  --namespace gavin \
  --from-literal=OIDC_ISSUER_URL=https://your-oidc-provider.com \
  --from-literal=OIDC_CLIENT_ID=your-client-id \
  --from-literal=OIDC_CLIENT_SECRET=your-client-secret \
  --from-literal=OIDC_REDIRECT_URL=https://gavin.restanrm.fr/api/auth/callback \
  --from-literal=SESSION_SECRET=$(openssl rand -hex 32) \
  --dry-run=client -o yaml | \
  kubeseal -o yaml > sealed-secret.yaml

# Commit sealed-secret.yaml to git
git add sealed-secret.yaml
git commit -m "Add sealed secrets for gavin"
```

## Configuration

### OIDC Setup with Pocket ID

1. **Register Application in Pocket ID:**
   - Go to your Pocket ID admin panel
   - Create new OAuth2 client
   - Note the Client ID and Client Secret

2. **Configure Redirect URL:**
   
   The redirect URL must match your deployment:
   
   ```
   # For local development
   http://localhost:3000/api/auth/callback
   
   # For production with domain
   https://gavin.restanrm.fr/api/auth/callback
   
   # For Kubernetes port-forward
   http://localhost:3000/api/auth/callback
   ```

3. **Set Environment Variables:**
   
   All secrets must be set:
   - `OIDC_ISSUER_URL`: Your Pocket ID base URL
   - `OIDC_CLIENT_ID`: From Pocket ID
   - `OIDC_CLIENT_SECRET`: From Pocket ID
   - `OIDC_REDIRECT_URL`: Callback URL (must match!)
   - `SESSION_SECRET`: Random 64+ character string

### Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `HOST` | No | `0.0.0.0` | Listen address |
| `PORT` | No | `3000` | Listen port |
| `DATABASE_URL` | No | `sqlite:///app/data/gavin.db` | Database connection string |
| `UPLOAD_DIR` | No | `/app/data/uploads` | Upload directory path; cached album-cover thumbnails are stored in `album-covers/` below this directory |
| `FRONTEND_DIR` | No | `/app/dist` | Frontend static files path |
| `PUBLIC_DOMAIN` | No | `gavin.restanrm.fr` | Public domain used for default callback URLs |
| `AUTH_MODE` | No | `oidc` | `oidc` for production, `dev` for local auth bypass |
| `OIDC_ISSUER_URL` | Yes in OIDC mode | - | OIDC provider base URL |
| `OIDC_CLIENT_ID` | Yes in OIDC mode | - | OAuth2 client ID |
| `OIDC_CLIENT_SECRET` | Yes in OIDC mode | - | OAuth2 client secret |
| `OIDC_REDIRECT_URL` | No | `https://<PUBLIC_DOMAIN>/api/auth/callback` | OAuth2 callback URL |
| `SESSION_SECRET` | Yes in OIDC mode | - | Session encryption key (64+ chars) |
| `COOKIE_SECURE` | No | `true` | Use secure cookies (HTTPS only) |
| `ALBUM_METADATA_ENABLED` | No | `true` | Enable MusicBrainz/Cover Art Archive enrichment when albums are added |
| `ALBUM_METADATA_USER_AGENT` | No | generated | User agent for MusicBrainz requests; set this to identify your deployment |
| `ALBUM_COVER_RECOGNITION_PROVIDER` | No | auto | Album-cover visual recognition provider: `gemini`, `openai`, or `disabled` |
| `GEMINI_API_KEY` | Required when provider is `gemini` | - | Gemini API key for album-cover recognition |
| `GEMINI_BASE_URL` | No | `https://generativelanguage.googleapis.com` | Gemini API base URL |
| `GEMINI_ALBUM_COVER_MODEL` | No | `gemini-2.0-flash` | Gemini vision model used for album-cover recognition |
| `OPENAI_API_KEY` | Required when provider is `openai` | - | OpenAI API key for ChatGPT album-cover recognition |
| `OPENAI_BASE_URL` | No | `https://api.openai.com` | OpenAI-compatible API base URL |
| `OPENAI_ALBUM_COVER_MODEL` | No | `gpt-4o-mini` | ChatGPT vision model used for album-cover recognition |
| `MUSICBRAINZ_BASE_URL` | No | `https://musicbrainz.org` | MusicBrainz API base URL |
| `COVER_ART_ARCHIVE_BASE_URL` | No | `https://coverartarchive.org` | Cover Art Archive base URL |
| `RUST_LOG` | No | `info,gavin=info` | Log level configuration |

## Troubleshooting

### Application Won't Start

**Check logs:**
```bash
# Podman
podman logs gavin

# Kubernetes
kubectl logs -n gavin -l app.kubernetes.io/name=gavin
```

**Common issues:**
- Missing OIDC environment variables
- Invalid OIDC configuration
- Database permission issues
- Port already in use

### OIDC Authentication Fails

1. **Verify redirect URL matches:**
   ```bash
   kubectl exec -n gavin deployment/gavin -- env | grep OIDC_REDIRECT_URL
   ```
   
   Must exactly match configured callback in Pocket ID.

2. **Check OIDC issuer is reachable:**
   ```bash
   curl https://your-oidc-provider.com/.well-known/openid-configuration
   ```

3. **Verify client credentials:**
   - Double-check Client ID and Secret in Pocket ID
   - Ensure they match environment variables

### Database Issues

**Check permissions:**
```bash
kubectl exec -n gavin deployment/gavin -- ls -la /app/data
```

**Check migrations:**
```bash
kubectl logs -n gavin -l app.kubernetes.io/name=gavin | grep migration
```

### Persistence Issues

**Check PVC status:**
```bash
kubectl get pvc -n gavin
kubectl describe pvc gavin -n gavin
```

**Check storage class:**
```bash
kubectl get storageclass
```

### Health Check Failures

**Manual health check:**
```bash
# Podman
curl http://localhost:3000/api/health

# Kubernetes
kubectl port-forward -n gavin svc/gavin 8080:80
curl http://localhost:8080/api/health
```

Expected response:
```json
{"status":"ok"}
```

### Image Pull Errors

**Check image exists:**
```bash
podman pull your-registry.io/gavin:0.1.0
```

**Check imagePullSecrets:**
```bash
kubectl get secret -n gavin
kubectl describe deployment gavin -n gavin | grep -A5 Image
```

### Resource Constraints

**Check resource usage:**
```bash
kubectl top pods -n gavin
kubectl describe pod -n gavin -l app.kubernetes.io/name=gavin
```

**Adjust resources:**
```bash
helm upgrade gavin ./charts/gavin \
  --namespace gavin \
  --reuse-values \
  --set resources.limits.memory=1Gi \
  --set resources.limits.cpu=1000m
```

## Monitoring

### Prometheus Metrics (Future)

The application currently provides a health endpoint. Future versions will expose Prometheus metrics.

### Log Aggregation

Configure log shipping:

```yaml
# In values.yaml
podAnnotations:
  fluentbit.io/parser: json
```

### Alerting Rules Example

```yaml
groups:
  - name: gavin
    interval: 30s
    rules:
      - alert: GavinDown
        expr: up{job="gavin"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Gavin application is down"
```

## Backup and Recovery

### Backup SQLite Database

```bash
# Podman
podman exec gavin sqlite3 /app/data/gavin.db ".backup '/app/data/gavin-backup.db'"
podman cp gavin:/app/data/gavin-backup.db ./backup/

# Kubernetes
kubectl exec -n gavin deployment/gavin -- \
  sqlite3 /app/data/gavin.db ".backup '/app/data/gavin-backup.db'"
kubectl cp gavin/gavin-pod:/app/data/gavin-backup.db ./backup/
```

### Restore from Backup

```bash
# Stop application
kubectl scale deployment gavin -n gavin --replicas=0

# Copy backup
kubectl cp ./backup/gavin-backup.db gavin/gavin-pod:/app/data/gavin.db

# Restart
kubectl scale deployment gavin -n gavin --replicas=1
```

### PVC Snapshot (if supported)

```bash
kubectl get volumesnapshot -n gavin
```

## Security Best Practices

1. **Secrets Management:**
   - Use external secret management (Vault, External Secrets Operator)
   - Rotate secrets regularly
   - Never commit secrets to git

2. **Network Security:**
   - Enable NetworkPolicy in production
   - Use TLS/HTTPS for all external traffic
   - Restrict ingress to necessary sources

3. **Container Security:**
   - Scan images for vulnerabilities
   - Run as non-root user (default in our Containerfile/Dockerfile)
   - Use read-only root filesystem where possible

4. **RBAC:**
   - Use least-privilege service accounts
   - Audit permissions regularly

## Performance Tuning

### Database

SQLite is suitable for small to medium deployments. For large scale:
- Monitor database size
- Consider backup schedule
- Vacuum database periodically

### Caching

Frontend assets are built-in. For high traffic:
- Use CDN for static assets
- Configure nginx caching for uploads
- Consider Redis for sessions (future enhancement)

### Scaling

The default deployment keeps `replicaCount: 1` because SQLite is the intended database. For future horizontal scaling, first move the catalog database to a multi-writer service (for example PostgreSQL), then revisit HPA, shared object storage for uploads, and an external session store.

---

For more information, see the [main README](../README.md) and [Helm chart documentation](../charts/gavin/README.md).
