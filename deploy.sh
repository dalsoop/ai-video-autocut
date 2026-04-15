#!/usr/bin/env bash
# autocut-tui 빌드 + 배포
# 사용법: ./deploy.sh [vmid=50064]
set -euo pipefail
VMID="${1:-50064}"
BIN=target/release/autocut-tui

cd "$(dirname "$0")"

echo "=== 빌드 (release) ==="
cargo build --release

echo "=== 체크섬 ==="
sha256sum "$BIN"
size=$(stat -c%s "$BIN")
echo "크기: $((size/1024))KB"

echo "=== 배포 → LXC $VMID:/usr/local/bin/autocut-tui ==="
pct push "$VMID" "$BIN" /usr/local/bin/autocut-tui
pct exec "$VMID" -- chmod +x /usr/local/bin/autocut-tui
pct exec "$VMID" -- ls -la /usr/local/bin/autocut-tui

echo "=== 설정 템플릿 ==="
pct exec "$VMID" -- bash -c 'mkdir -p /root/.config/autocut
if [ ! -f /root/.config/autocut/config.ncl ]; then
cat > /root/.config/autocut/config.ncl <<EOF
{
  endpoint = "http://localhost:8080",
  defaults = {
    engine = "qwen3",
    lang = "Korean",
    whisper_model = "medium",
  },
}
EOF
echo "config 생성됨: /root/.config/autocut/config.ncl"
fi'

echo "완료. 사용: pct enter $VMID && autocut-tui"
