# T501 / Tomate MTM-1106 driver for Linux

A small userspace driver for drawing tablets that show up as:

```text
08f2:6811 Gotop Information Inc. [T501] Driver Inside Tablet
```

I made this for my own Tomate MTM-1106 because Linux only used the small
phone area of the tablet. I am sharing it in case someone else has the same
hardware.

This is a personal fix, not an actively maintained driver. If it keeps working
for me, I probably will not update it. Feel free to use or change it if it helps
with your tablet.

So far, it has only been tested on **Bazzite 43**, using **KDE Plasma/Wayland
in Desktop Mode**.

## What works

- the full drawing area;
- pressure sensitivity;
- absolute pen positioning;
- pen buttons and tablet shortcuts;
- automatic start when the tablet is connected;
- automatic stop when it is unplugged.

Fast diagonal lines are sent as a single input frame, which avoids the
stair-step lines I was seeing in Xournal++.

## Check your tablet

This was only tested with USB ID `08f2:6811`. Check yours with:

```bash
lsusb -d 08f2:6811
```

The product name is not enough. Other tablets may use different hardware even
if they look the same.

## Build on Bazzite

This builds the driver inside a temporary Fedora container, so Rust does not
need to be installed on the Bazzite host:

```bash
cd /path/to/t501-tablet-linux-driver

podman run --rm \
  -v "$PWD:/work:Z" \
  -w /work/driver \
  registry.fedoraproject.org/fedora:43 \
  bash -lc 'dnf install -y cargo rustfmt clippy libusb1-devel && \
    cargo test --all-targets --locked && \
    cargo clippy --all-targets --locked -- -D warnings && \
    cargo build --release --locked'
```

The binary will be at `driver/target/release/v1060p`. The old upstream name is
kept in the build, but it is installed below as `tomate-mtm1106-driver`.

## Install on Bazzite KDE

Run from the repository root:

```bash
install -Dm755 driver/target/release/v1060p \
  "$HOME/.local/libexec/tomate-mtm1106-driver"

install -Dm644 packaging/tomate-mtm1106.service \
  "$HOME/.config/systemd/user/tomate-mtm1106.service"

systemctl --user daemon-reload
systemctl --user enable tomate-mtm1106.service

pkexec install -Dm644 packaging/60-tomate-mtm1106.rules \
  /etc/udev/rules.d/60-tomate-mtm1106.rules

pkexec udevadm control --reload-rules
```

Unplug and reconnect the tablet. If it was already connected when Plasma
started, run:

```bash
systemctl --user start tomate-mtm1106.service
```

The service stays inactive while the tablet is not connected. To check it:

```bash
systemctl --user status tomate-mtm1106.service
journalctl --user -u tomate-mtm1106.service -n 30 --no-pager
```

## Remove

```bash
systemctl --user disable --now tomate-mtm1106.service
rm -f "$HOME/.config/systemd/user/tomate-mtm1106.service"
rm -f "$HOME/.local/libexec/tomate-mtm1106-driver"
systemctl --user daemon-reload

pkexec rm -f /etc/udev/rules.d/60-tomate-mtm1106.rules
pkexec udevadm control --reload-rules
```

## A few notes

- The driver runs as the desktop user, not as root.
- The `udev` rule only matches USB device `08f2:6811` and uses `uaccess`.
- The driver does not use the network or run external commands.
- The included service is tied to Plasma, so it does not start in Bazzite Game
  Mode.
- This is a userspace workaround, not a kernel driver.

I wrote down the safety checks I made in [AUDIT.md](AUDIT.md).

## Credits and license

This version was built with OpenAI Codex and tested on my own tablet.

This is based on
[feveal/Tablet-VINSA-1060-Plus-Linux-Driver](https://github.com/feveal/Tablet-VINSA-1060-Plus-Linux-Driver)
and the T501 work from
[DIGImend/10moons-tools](https://github.com/DIGImend/10moons-tools).

Licensed under GPL-3.0. See [LICENSE](LICENSE).
