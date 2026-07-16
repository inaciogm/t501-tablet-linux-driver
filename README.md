# T501 / Tomate MTM-1106 Linux userspace driver

Userspace driver for the screenless drawing tablet identified as `08f2:6811`
(`Gotop/SZ PENG YI T501`). This USB hardware is sold under names including
**Tomate MTM-1106** and **VINSA 1060 Plus**.

I originally made this adaptation to get my own tablet working correctly on
Bazzite. I am sharing it in case it helps someone else with the same hardware.
There is no guarantee that it will work with other models or operating systems.

> **Important:** this project has only been tested on **Bazzite 43**, in a
> **KDE Plasma/Wayland Desktop Mode** session. Other Linux distributions have
> not been tested.

## What it does

The driver:

- enables the full drawing area instead of the small phone/Android area;
- reads coordinates and pressure directly from the T501 USB interface;
- creates a virtual Linux pen and shortcut pad through `uinput`;
- starts in absolute mode for drawing;
- sends X, Y, pressure, proximity, touch, and pen buttons together per sample;
- starts automatically when the tablet is connected;
- exits and releases the USB port when the tablet is removed;
- does not keep scanning for devices while the tablet is absent.

Batching related events also prevents the stair-step diagonals produced when X
and Y were previously sent as separate input frames.

## Supported hardware

The tablet must have this exact USB identifier:

```text
08f2:6811 Gotop Information Inc. [T501] Driver Inside Tablet
```

Check it with:

```bash
lsusb -d 08f2:6811
```

A similar product name alone is not enough. This project does not claim support
for Huion tablets or devices with other USB IDs.

## Why it is needed

On this tablet, the generic Linux HID driver recognizes only the small
phone/Android interface. The full drawing area requires a different
initialization sequence and 64-byte reports.

This program sends the complete four-report sequence researched by the
DIGImend/10moons project, resets the USB device in the required order, reads
the full-area interface, and converts its data into Linux pen events.

## Security

- The reader runs as a user service, never as root.
- The `udev` rule grants access only to the active local user.
- The rule matches only USB device `08f2:6811`.
- No executable is launched inside the `udev` worker.
- The service has no infinite restart loop.
- The driver does not access the Internet, personal files, credentials, or
  external commands.
- It does not use unsafe world-writable permissions such as `MODE="0666"`.
- The service has additional filesystem, privilege, namespace, and
  network-family restrictions.

Software with access to `uinput` can synthesize input, so the source and
permissions should still be reviewed before installation. See
[AUDIT.md](AUDIT.md) for the audit record, dependency review, and reference
build hash.

## Build on Bazzite

The following command builds inside a disposable Fedora 43 container. Rust and
the build tools are installed only inside that container and are not layered
onto the immutable Bazzite system:

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

The compiled executable is written to `driver/target/release/v1060p`. That
upstream target name is kept for compatibility; the installation command gives
it a Tomate-specific name.

## Install on Bazzite KDE

Review the files under `packaging/` first. Then run these commands from the
repository root:

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
started, start the reader once with:

```bash
systemctl --user start tomate-mtm1106.service
```

It is normal for the service to remain inactive while the tablet is absent.

## Verify the installation

```bash
systemctl --user status tomate-mtm1106.service
journalctl --user -u tomate-mtm1106.service -n 30 --no-pager
```

After the first pen movement, the log should contain:

```text
Pen input is ready.
```

The following were verified on a Tomate MTM-1106:

- full-screen corner reach;
- pressure variation;
- precise slow movement;
- rapid diagonal strokes in Xournal++ 1.3.5 Flatpak;
- automatic exit when the tablet is removed;
- automatic start after reconnection;
- one reader process with no repeated restarts.

## Uninstall

```bash
systemctl --user disable --now tomate-mtm1106.service
rm -f "$HOME/.config/systemd/user/tomate-mtm1106.service"
rm -f "$HOME/.local/libexec/tomate-mtm1106-driver"
systemctl --user daemon-reload

pkexec rm -f /etc/udev/rules.d/60-tomate-mtm1106.rules
pkexec udevadm control --reload-rules
```

## Known limitations

- Tested only on Bazzite 43 with KDE Plasma/Wayland.
- Tested only with USB ID `08f2:6811`.
- The packaged service requires the Plasma target so it cannot launch in
  Bazzite Game Mode.
- Physical shortcut mappings are still defined directly in the source code.
- This is a userspace solution, not an in-kernel DIGImend driver.

## Lineage and license

This fork preserves the history and authorship of
`feveal/Tablet-VINSA-1060-Plus-Linux-Driver`, which also builds on earlier
community research. The full-area initialization sequence is derived from the
GPL `DIGImend/10moons-tools` project.

The code is distributed under the **GPL-3.0** license. See [LICENSE](LICENSE).
