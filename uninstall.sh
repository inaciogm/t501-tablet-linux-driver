#!/usr/bin/env bash
set -euo pipefail

echo "==> Desinstalando driver Tomate MTM-1106..."

systemctl --user disable --now tomate-mtm1106.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/tomate-mtm1106.service"
rm -f "$HOME/.local/libexec/tomate-mtm1106-driver"
systemctl --user daemon-reload

SUDO_CMD=""
if command -v pkexec >/dev/null 2>&1; then
    SUDO_CMD="pkexec"
elif command -v sudo >/dev/null 2>&1; then
    SUDO_CMD="sudo"
fi

if [ -n "$SUDO_CMD" ] && [ -f /etc/udev/rules.d/60-tomate-mtm1106.rules ]; then
    $SUDO_CMD rm -f /etc/udev/rules.d/60-tomate-mtm1106.rules
    $SUDO_CMD udevadm control --reload-rules
fi

echo "Desinstalação concluída."
