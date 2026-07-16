# Safety notes

These are the checks I made before using the driver on my computer. They are
not a guarantee, but they may be useful to anyone reviewing the project.

## Source

- Hardware: T501 / Tomate MTM-1106, USB `08f2:6811`.
- Upstream project: `feveal/Tablet-VINSA-1060-Plus-Linux-Driver`.
- Upstream commit reviewed: `aedea60d2ab4df2e4895346217f63a69cd8fdf7e`.
- License: GPL-3.0.
- Full-area initialization is based on the DIGImend `10moons-tools` work.

## What I checked

The Rust code does not contain network access, external command execution,
credential access, dynamic code loading, or `unsafe` blocks.

It does two important things:

1. Talks to USB device `08f2:6811` and reads its pen reports.
2. Creates virtual input devices through `/dev/uinput`.

Access to `uinput` is sensitive because it can create keyboard or pointer
events. The included rule only grants access to the logged-in user and only
matches this tablet. It does not use `MODE="0666"`.

`Cargo.lock` contains 26 crates.io packages and no Git dependencies. On
2026-07-16, `cargo audit` found no known RustSec vulnerabilities. These checks
also passed:

```text
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

The binary built in the Fedora 43 container had this SHA-256:

```text
3b49e8d30d89b1c3171620cbb57f7b8ffa69ec3a4ba1749bbb022fc5ea836af8
```

Run the checks again if the code, dependencies, or build environment change.

## Main changes from upstream

- Use the full four-report initialization sequence.
- Start once when the tablet is connected and stop when it is removed.
- Send each pen sample in one evdev batch to fix stair-step diagonals.
- Use an exact-device `uaccess` rule instead of world-writable permissions.
- Use one systemd user service instead of the old desktop launcher.
- Declare pressure, touch, proximity, axes, and pen buttons correctly.

## References

- <https://github.com/feveal/Tablet-VINSA-1060-Plus-Linux-Driver>
- <https://github.com/DIGImend/10moons-tools>
- <https://github.com/DIGImend/digimend-kernel-drivers/issues/722>
- <https://github.com/DIGImend/digimend-kernel-drivers/issues/182>
