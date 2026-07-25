#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CLUSTER_NAME="${CLUSTER_NAME:-gavin-test}"
NAMESPACE="${NAMESPACE:-gavin-dev}"
IMAGE="${IMAGE:-docker.io/library/gavin:k3d-test}"
KUBECONFIG_CONTEXT="k3d-${CLUSTER_NAME}"
PODMAN_SOCKET="${PODMAN_SOCKET:-${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/podman/podman.sock}"
export DOCKER_HOST="${DOCKER_HOST:-unix://${PODMAN_SOCKET}}"

for cmd in podman k3d kubectl helm; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

if [[ ! -S "$PODMAN_SOCKET" ]]; then
  echo "Podman socket not found at $PODMAN_SOCKET" >&2
  echo "Start it with: systemctl --user start podman.socket" >&2
  exit 1
fi

if ! k3d cluster list --no-headers 2>/dev/null | awk '{print $1}' | grep -qx "$CLUSTER_NAME"; then
  echo "Creating k3d cluster '$CLUSTER_NAME' using Podman's Docker-compatible socket..."
  k3d cluster create "$CLUSTER_NAME" --servers 1 --agents 0 --wait
else
  echo "Using existing k3d cluster '$CLUSTER_NAME'."
fi

kubectl config use-context "$KUBECONFIG_CONTEXT" >/dev/null

echo "Building image with Podman: $IMAGE"
podman build -t "$IMAGE" .

image_tar="$(mktemp --suffix=.tar)"
cleanup() {
  rm -f "$image_tar"
}
trap cleanup EXIT

echo "Importing image into k3d cluster '$CLUSTER_NAME'..."
podman save "$IMAGE" -o "$image_tar"
k3d image import "$image_tar" --cluster "$CLUSTER_NAME"

echo "Deploying Gavin in dev auth mode to namespace '$NAMESPACE'..."
helm upgrade --install gavin ./charts/gavin \
  --namespace "$NAMESPACE" \
  --create-namespace \
  --set image.repository="${IMAGE%:*}" \
  --set image.tag="${IMAGE##*:}" \
  --set image.pullPolicy=IfNotPresent \
  --set config.authMode=dev \
  --set config.cookieSecure=false \
  --set persistence.enabled=false \
  --set secrets.sessionSecret=dev-secret-not-secure-at-least-64-characters-long-for-session-key

kubectl rollout status deployment/gavin -n "$NAMESPACE" --timeout=180s

cat <<EOF

Gavin is running in k3d dev mode.

Port-forward it with:
  kubectl port-forward -n $NAMESPACE svc/gavin 8080:80

Then open:
  http://127.0.0.1:8080

Useful cleanup:
  helm uninstall gavin -n $NAMESPACE
  k3d cluster delete $CLUSTER_NAME
EOF
