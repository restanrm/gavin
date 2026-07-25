# Gavin Helm Chart

This Helm chart deploys the Gavin Vinyl Library application on Kubernetes.

Gavin uses SQLite by design. Keep `replicaCount: 1` unless the database layer changes.

## Prerequisites

- Kubernetes 1.20+
- Helm 3.8+
- PersistentVolume provisioner support in the cluster (if persistence is enabled)
- A configured OIDC provider (e.g., Pocket ID)

## Installing the Chart

### Basic Installation

```bash
# Add your custom values
helm install gavin ./charts/gavin \
  --set secrets.oidcIssuerUrl=https://your-oidc-provider.com \
  --set secrets.oidcClientId=your-client-id \
  --set secrets.oidcClientSecret=your-client-secret \
  --set secrets.oidcRedirectUrl=https://gavin.restanrm.fr/api/auth/callback \
  --set secrets.sessionSecret=$(openssl rand -hex 32)
```

### Using an Existing Secret (Recommended)

For production, create a secret externally (e.g., using Sealed Secrets, External Secrets Operator, or manually):

```bash
kubectl create secret generic gavin-secrets \
  --from-literal=OIDC_ISSUER_URL=https://your-oidc-provider.com \
  --from-literal=OIDC_CLIENT_ID=your-client-id \
  --from-literal=OIDC_CLIENT_SECRET=your-client-secret \
  --from-literal=OIDC_REDIRECT_URL=https://gavin.restanrm.fr/api/auth/callback \
  --from-literal=SESSION_SECRET=$(openssl rand -hex 32)

helm install gavin ./charts/gavin \
  --set existingSecret=gavin-secrets
```

### With Ingress

```bash
helm install gavin ./charts/gavin \
  -f values-prod.yaml \
  --set ingress.enabled=true \
  --set ingress.className=nginx \
  --set ingress.hosts[0].host=gavin.restanrm.fr \
  --set ingress.hosts[0].paths[0].path=/ \
  --set ingress.hosts[0].paths[0].pathType=Prefix \
  --set ingress.tls[0].secretName=gavin-tls \
  --set ingress.tls[0].hosts[0]=gavin.restanrm.fr
```

## Configuration

The following table lists the configurable parameters and their default values.

### Global Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of replicas. Keep `1` with SQLite. | `1` |
| `domain` | Default public domain used for ingress/callback examples | `gavin.restanrm.fr` |
| `image.repository` | Image repository | `gavin` |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `image.tag` | Image tag (defaults to chart appVersion) | `""` |

### Service Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `service.type` | Service type | `ClusterIP` |
| `service.port` | Service port | `80` |
| `service.targetPort` | Container port | `3000` |

### Ingress Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `ingress.enabled` | Enable ingress | `false` |
| `ingress.className` | Ingress class name | `""` |
| `ingress.hosts` | Ingress hosts configuration (empty host uses `domain`) | `[domain]` |
| `ingress.tls` | Ingress TLS configuration | `[]` |

### Persistence Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `persistence.enabled` | Enable persistence | `true` |
| `persistence.existingClaim` | Use existing PVC | `""` |
| `persistence.storageClass` | Storage class | `""` |
| `persistence.accessModes` | Access modes | `[ReadWriteOnce]` |
| `persistence.size` | Volume size | `5Gi` |
| `persistence.mountPath` | Mount path | `/app/data` |

### Application Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `config.host` | Listen host | `0.0.0.0` |
| `config.port` | Listen port | `3000` |
| `config.databaseUrl` | Database URL | `sqlite:///app/data/gavin.db` |
| `config.publicDomain` | Override app `PUBLIC_DOMAIN` (empty uses `domain`) | `""` |
| `config.authMode` | Authentication mode (`oidc` or `dev`) | `oidc` |
| `config.cookieSecure` | Secure cookies | `true` |
| `config.albumMetadataEnabled` | Enable MusicBrainz/Cover Art Archive metadata enrichment | `true` |
| `config.albumMetadataUserAgent` | Optional metadata lookup user agent | `""` |
| `config.albumCoverRecognitionProvider` | Cover recognition provider (`gemini`, `openai`, or `disabled`) | `gemini` |
| `config.geminiBaseUrl` | Gemini API base URL for cover recognition | `https://generativelanguage.googleapis.com` |
| `config.geminiAlbumCoverModel` | Gemini vision model used for album-cover recognition | `gemini-2.0-flash` |
| `config.openaiBaseUrl` | OpenAI-compatible API base URL for ChatGPT cover recognition | `https://api.openai.com` |
| `config.openaiAlbumCoverModel` | ChatGPT vision model used for album-cover recognition | `gpt-4o-mini` |
| `config.musicbrainzBaseUrl` | MusicBrainz base URL | `https://musicbrainz.org` |
| `config.coverArtArchiveBaseUrl` | Cover Art Archive base URL | `https://coverartarchive.org` |

### Security Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `existingSecret` | Name of existing secret for OIDC/session | `""` |
| `secrets.oidcIssuerUrl` | OIDC issuer URL | `""` |
| `secrets.oidcClientId` | OIDC client ID | `""` |
| `secrets.oidcClientSecret` | OIDC client secret | `""` |
| `secrets.oidcRedirectUrl` | OIDC redirect URL (empty generates `https://<domain>/api/auth/callback`) | `""` |
| `secrets.sessionSecret` | Session secret (64+ chars) | `""` |
| `secrets.geminiApiKey` | Gemini API key for album-cover recognition | `""` |
| `secrets.openaiApiKey` | OpenAI API key for ChatGPT album-cover recognition | `""` |

### Resources

| Parameter | Description | Default |
|-----------|-------------|---------|
| `resources.limits.cpu` | CPU limit | `500m` |
| `resources.limits.memory` | Memory limit | `512Mi` |
| `resources.requests.cpu` | CPU request | `100m` |
| `resources.requests.memory` | Memory request | `256Mi` |

## Upgrading

```bash
helm upgrade gavin ./charts/gavin -f your-values.yaml
```

## Uninstalling

```bash
helm uninstall gavin
```

**Note**: This will not delete the PVC by default. To delete the PVC:

```bash
kubectl delete pvc gavin
```

## ArgoCD Integration

This chart is ArgoCD-friendly. Example Application manifest:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: gavin
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/restanrm/gavin
    targetRevision: main
    path: charts/gavin
    helm:
      valueFiles:
        - values-prod.yaml
  destination:
    server: https://kubernetes.default.svc
    namespace: gavin
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
      - CreateNamespace=true
```

## Troubleshooting

### Pods not starting

```bash
kubectl describe pod -l app.kubernetes.io/name=gavin
kubectl logs -l app.kubernetes.io/name=gavin
```

### Database persistence issues

Ensure your cluster has a working PV provisioner:

```bash
kubectl get storageclass
kubectl get pv
kubectl get pvc
```

### OIDC authentication not working

Verify secrets are correctly set:

```bash
kubectl get secret gavin -o yaml
kubectl exec -it deploy/gavin -- env | grep OIDC
```

Ensure your OIDC provider has the correct redirect URL configured.
