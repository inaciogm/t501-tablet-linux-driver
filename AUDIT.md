# Audit record

## Scope and provenance

- Target hardware: Gotop/SZ PENG YI T501, USB `08f2:6811`.
- Confirmed rebrand: Tomate MTM-1106 (pressure tablet without a display).
- Upstream repository: `feveal/Tablet-VINSA-1060-Plus-Linux-Driver`.
- Reviewed upstream commit: `aedea60d2ab4df2e4895346217f63a69cd8fdf7e`.
- License: GPL-3.0, preserved in this fork.
- Full-area protocol source: DIGImend `10moons-tools` and the related T501
  hardware investigations.

## Code review

The Rust driver is small (three source files, about 700 lines after the local
changes). A manual review found no network client, subprocess execution,
credential access, arbitrary filesystem access, deletion logic, dynamic code
loading, or Rust `unsafe` blocks.

The program performs two privileged-in-effect operations as the desktop user:

1. It sends four fixed USB feature reports only to device `08f2:6811`, claims
   that device's interfaces, and reads interrupt endpoint `0x84`.
2. It creates a virtual pen and keyboard through `/dev/uinput`.

The second capability is inherently sensitive: a malicious program with the
same access could synthesize keyboard input. For that reason, popularity or a
successful antivirus scan is not sufficient evidence of safety.

## Dependency review

The lockfile contains 26 Rust packages and only crates.io registry sources; it
contains no Git dependencies. On 2026-07-16, `cargo audit` checked the locked
dependency graph against 1,160 RustSec advisories and reported no known
vulnerabilities. The following also passed with the locked dependency set:

```text
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

The reference build produced in the Fedora 43 container with Cargo/Rust
1.96.1 has SHA-256:

```text
3b49e8d30d89b1c3171620cbb57f7b8ffa69ec3a4ba1749bbb022fc5ea836af8
```

This is a point-in-time review, not a permanent guarantee. Re-run the checks
after changing code, dependencies, compiler, or build environment.

## Findings addressed in this fork

- The upstream README granted world read/write access (`0666`) to the tablet,
  all input devices, and `uinput`. Those instructions are not retained.
- The upstream Rust code sent only one of the four DIGImend initialization
  reports and reset in an order that made full-area startup timing-dependent.
- The original loop could repeatedly scan and reconnect. This fork initializes
  once for each add event and exits when the device disappears.
- The original virtual pen emitted undeclared touch events and lacked a pen
  proximity capability, which could fail on current kernels/compositors.
- The original virtual pen emitted X, Y, and pressure in separate `evdev`
  batches. Because `VirtualDevice::emit()` appends `SYN_REPORT`, diagonal
  samples appeared as horizontal and vertical steps in strict drawing apps.
- The old desktop launcher could race a second copy of the reader and was
  removed in favor of one systemd user service.

## Runtime containment

The included USB rule matches only `08f2:6811`, uses `TAG+="uaccess"`, and
starts no executable in the udev worker. The service runs under the active
user, has no restart loop, cannot write to the home or system filesystem, and
is restricted from Internet address families. It requires the Plasma workspace
and is stopped with it, so the packaged Bazzite setup does not run in Game Mode.

## References

- <https://github.com/feveal/Tablet-VINSA-1060-Plus-Linux-Driver>
- <https://github.com/DIGImend/10moons-tools>
- <https://github.com/DIGImend/digimend-kernel-drivers/issues/722>
- <https://github.com/DIGImend/digimend-kernel-drivers/issues/182>
