# Safety notes

These are the security and stability checks made before running the driver on Linux.

## Source & Architecture

- Hardware: T501 / Tomate MTM-1106, USB `08f2:6811`.
- Upstream project: `feveal/Tablet-VINSA-1060-Plus-Linux-Driver`.
- Upstream commit reviewed: `aedea60d2ab4df2e4895346217f63a69cd8fdf7e`.
- License: GPL-3.0.
- Full-area initialization is based on the DIGImend `10moons-tools` work.

## Security Verification

The Rust code does not contain network access, external command execution,
credential access, dynamic code loading, or `unsafe` blocks.

It does two specific operations:

1. Communicates directly with USB device `08f2:6811` to read raw pen reports via `rusb`.
2. Creates virtual input devices through `/dev/uinput` via `evdev`.

Access to `uinput` is restricted: the included `udev` rule only grants access to
the active logged-in user via `TAG+="uaccess"` and only matches the exact USB ID
`08f2:6811`. It does not use world-writable `MODE="0666"`.

`Cargo.lock` contains 26 crates.io packages and no Git dependencies.
These automated checks pass:

```text
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
cargo build --release --locked
```

The release binary built in the container has this SHA-256:

```text
30c95b2f01fdabc3573ea022e5bdd88bf0be13fa7f8210e3afebb27317d5156c
```

## Main Changes from Upstream

- Send the complete four-report feature sequence to unlock full 10x6" area.
- Add physical contact hysteresis (1480 enter / 1600 exit) to eliminate hover blinking.
- Support dual operation: Pen Mode (drawing/stylus) and Scroll Mode (kinetic drag-to-scroll with tap-to-click).
- Coalesce diagonal coordinates into single evdev frames with `BTN_TOUCH` emitted first.
- Reduce USB read timeout to 60 ms and cleanly emit proximity release on timeout.
- Unprivileged user service and exact-device `udev` rule instead of global root daemons.

## References

- <https://github.com/feveal/Tablet-VINSA-1060-Plus-Linux-Driver>
- <https://github.com/DIGImend/10moons-tools>
- <https://github.com/DIGImend/digimend-kernel-drivers/issues/722>
- <https://github.com/DIGImend/digimend-kernel-drivers/issues/182>
