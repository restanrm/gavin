# ArgoCD Deployment Examples

This directory contains example ArgoCD Application manifests for deploying Gavin Vinyl Library.

## Prerequisites

- ArgoCD installed in your cluster
- Repository accessible by ArgoCD
- Secrets configured (see options below)

## Examples

### 1. Basic Application (`application.yaml`)

Simple ArgoCD Application with standard Kubernetes Secret.

**Setup:**
```bash
# Create namespace and secret
kubectl create namespace gavin
kubectl create secret generic gavin-secrets \
  --namespace gavin \
  --from-literal=OIDC_ISSUER_URL=https://your-oidc-provider.com \
  --from-literal=OIDC_CLIENT_ID=your-client-id \
  --from-literal=OIDC_CLIENT_SECRET=your-client-secret \
  --from-literal=OIDC_REDIRECT_URL=https://gavin.restanrm.fr/api/auth/callback \
  --from-literal=SESSION_SECRET=$(openssl rand -hex 32)

# Deploy ArgoCD application
kubectl apply -f application.yaml
```

**Pros:**
- Simple and straightforward
- No additional components needed

**Cons:**
- Secrets not in Git (must be created manually)
- No GitOps for secrets

---

### 2. External Secrets Operator (`application-external-secrets.yaml`)

Uses External Secrets Operator to sync from external secret stores (AWS Secrets Manager, Vault, GCP Secret Manager, etc.).

**Prerequisites:**
```bash
# Install External Secrets Operator
helm repo add external-secrets https://charts.external-secrets.io
helm install external-secrets \
  external-secrets/external-secrets \
  -n external-secrets-system \
  --create-namespace
```

**Setup:**

1. **Store secrets in AWS Secrets Manager:**
   ```bash
   # Create OIDC secret
   aws secretsmanager create-secret \
     --name gavin/oidc \
     --secret-string '{
       "issuer_url":"https://your-oidc-provider.com",
       "client_id":"your-client-id",
       "client_secret":"your-client-secret",
       "redirect_url":"https://gavin.restanrm.fr/api/auth/callback"
     }'
   
   # Create session secret
   aws secretsmanager create-secret \
     --name gavin/session \
     --secret-string "{\"secret\":\"$(openssl rand -hex 32)\"}"
   ```

2. **Configure IAM role for service account (IRSA):**
   ```bash
   # Create IAM policy
   aws iam create-policy \
     --policy-name GavinSecretsAccess \
     --policy-document '{
       "Version": "2012-10-17",
       "Statement": [{
         "Effect": "Allow",
         "Action": ["secretsmanager:GetSecretValue"],
         "Resource": ["arn:aws:secretsmanager:*:*:secret:gavin/*"]
       }]
     }'
   
   # Associate IAM role with service account
   eksctl create iamserviceaccount \
     --name external-secrets-sa \
     --namespace gavin \
     --cluster your-cluster \
     --attach-policy-arn arn:aws:iam::ACCOUNT_ID:policy/GavinSecretsAccess \
     --approve
   ```

3. **Deploy:**
   ```bash
   kubectl apply -f application-external-secrets.yaml
   ```

**Pros:**
- Secrets stored in secure external system
- Full GitOps workflow
- Automatic secret rotation support
- Audit trail in secret manager

**Cons:**
- Requires external secret store
- Additional complexity
- Cloud provider dependency

---

### 3. Sealed Secrets (`application-sealed-secrets.yaml`)

Uses Bitnami Sealed Secrets for GitOps-safe secret encryption.

**Prerequisites:**
```bash
# Install Sealed Secrets controller
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/controller.yaml

# Install kubeseal CLI
wget https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/kubeseal-0.24.0-linux-amd64.tar.gz
tar xfz kubeseal-0.24.0-linux-amd64.tar.gz
sudo install -m 755 kubeseal /usr/local/bin/kubeseal
```

**Setup:**

1. **Create and seal secrets:**
   ```bash
   # Create regular secret
   kubectl create secret generic gavin-secrets \
     --namespace gavin \
     --from-literal=OIDC_ISSUER_URL=https://your-oidc-provider.com \
     --from-literal=OIDC_CLIENT_ID=your-client-id \
     --from-literal=OIDC_CLIENT_SECRET=your-client-secret \
     --from-literal=OIDC_REDIRECT_URL=https://gavin.restanrm.fr/api/auth/callback \
     --from-literal=SESSION_SECRET=$(openssl rand -hex 32) \
     --dry-run=client -o yaml > secret.yaml
   
   # Seal the secret
   kubeseal -o yaml < secret.yaml > sealed-secret.yaml
   
   # Clean up plaintext secret
   rm secret.yaml
   ```

2. **Update the sealed secret in `application-sealed-secrets.yaml`:**
   Replace the encrypted values with your sealed secret values.

3. **Deploy:**
   ```bash
   kubectl apply -f application-sealed-secrets.yaml
   ```

**Pros:**
- Secrets safely stored in Git
- Full GitOps workflow
- No external dependencies (beyond controller)
- Works with any Kubernetes cluster

**Cons:**
- Sealed secrets tied to cluster (need re-encryption for different clusters)
- Manual secret rotation
- Must re-seal when updating secrets

---

## Comparison Matrix

| Feature | Basic | External Secrets | Sealed Secrets |
|---------|-------|------------------|----------------|
| GitOps-friendly | ❌ | ✅ | ✅ |
| External dependencies | None | Secret store | Controller |
| Secret rotation | Manual | Automatic | Manual |
| Multi-cluster | ✅ | ✅ | ⚠️ (re-seal) |
| Audit trail | ❌ | ✅ | ⚠️ (Git) |
| Setup complexity | Low | High | Medium |
| Runtime complexity | Low | Medium | Low |

---

## Customization

### Change Image

Edit the `image.repository` and `image.tag` parameters:

```yaml
helm:
  parameters:
    - name: image.repository
      value: "your-registry.io/gavin"
    - name: image.tag
      value: "0.1.0"
```

### Enable Ingress

Add ingress parameters:

```yaml
helm:
  parameters:
    - name: ingress.enabled
      value: "true"
    - name: ingress.className
      value: "nginx"
    - name: ingress.hosts[0].host
      value: "gavin.restanrm.fr"
    - name: ingress.hosts[0].paths[0].path
      value: "/"
    - name: ingress.hosts[0].paths[0].pathType
      value: "Prefix"
```

### Use Custom Values File

Instead of parameters, use a custom values file:

1. Create `values-custom.yaml` in your repo
2. Update the Application:
   ```yaml
   source:
     helm:
       valueFiles:
         - values-prod.yaml
         - values-custom.yaml  # Your overrides
   ```

---

## Monitoring ArgoCD Applications

```bash
# Get application status
argocd app get gavin

# Sync application
argocd app sync gavin

# View application logs
argocd app logs gavin

# View application resources
argocd app resources gavin

# Set application to manual sync
argocd app set gavin --sync-policy none

# Enable auto-sync
argocd app set gavin --sync-policy automated
```

---

## Troubleshooting

### Application stuck in Progressing

```bash
argocd app get gavin
kubectl describe application gavin -n argocd
kubectl get events -n gavin --sort-by='.lastTimestamp'
```

### Secrets not created

Check ExternalSecret or SealedSecret status:
```bash
kubectl describe externalsecret gavin-secrets -n gavin
kubectl describe sealedsecret gavin-secrets -n gavin
```

### Out of sync

```bash
# View differences
argocd app diff gavin

# Hard refresh
argocd app diff gavin --hard-refresh

# Force sync
argocd app sync gavin --force
```

---

For more information:
- [ArgoCD Documentation](https://argo-cd.readthedocs.io/)
- [External Secrets Operator](https://external-secrets.io/)
- [Sealed Secrets](https://github.com/bitnami-labs/sealed-secrets)
