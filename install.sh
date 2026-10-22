#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_SRC=""

if [ -f "$SCRIPT_DIR/driver/target/release/v1060p" ]; then
    BIN_SRC="$SCRIPT_DIR/driver/target/release/v1060p"
elif [ -f "$SCRIPT_DIR/dist/tomate-mtm1106-driver" ]; then
    BIN_SRC="$SCRIPT_DIR/dist/tomate-mtm1106-driver"
elif [ -f "$SCRIPT_DIR/tomate-mtm1106-driver" ]; then
    BIN_SRC="$SCRIPT_DIR/tomate-mtm1106-driver"
else
    echo "Erro: Binário do driver não encontrado." >&2
    echo "Compile com podman/cargo ou extraia o pacote de download na raiz do repositório." >&2
    exit 1
fi

echo "==> Instalando driver Tomate MTM-1106 (v1.1.0)..."

mkdir -p "$HOME/.local/libexec"
install -Dm755 "$BIN_SRC" "$HOME/.local/libexec/tomate-mtm1106-driver"

mkdir -p "$HOME/.config/systemd/user"
install -Dm644 "$SCRIPT_DIR/packaging/tomate-mtm1106.service" "$HOME/.config/systemd/user/tomate-mtm1106.service"

echo "==> Ativando serviço de usuário no systemd..."
systemctl --user daemon-reload
systemctl --user enable tomate-mtm1106.service

echo "==> Instalando regra do udev (solicitará autorização)..."
SUDO_CMD=""
if command -v pkexec >/dev/null 2>&1; then
    SUDO_CMD="pkexec"
elif command -v sudo >/dev/null 2>&1; then
    SUDO_CMD="sudo"
else
    echo "Aviso: pkexec/sudo não encontrados. Copie packaging/60-tomate-mtm1106.rules para /etc/udev/rules.d/ manualmente." >&2
fi

if [ -n "$SUDO_CMD" ]; then
    $SUDO_CMD install -Dm644 "$SCRIPT_DIR/packaging/60-tomate-mtm1106.rules" /etc/udev/rules.d/60-tomate-mtm1106.rules
    $SUDO_CMD udevadm control --reload-rules
    $SUDO_CMD udevadm trigger --subsystem-match=usb 2>/dev/null || true
fi

echo "==> Iniciando serviço..."
systemctl --user restart tomate-mtm1106.service || systemctl --user start tomate-mtm1106.service

echo ""
echo "Instalação concluída com sucesso!"
echo "Status do serviço:"
systemctl --user status tomate-mtm1106.service --no-pager || true
