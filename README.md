# Tomate MTM-1106 / Gotop T501 Linux Driver (v1.1.0)

A lightweight userspace driver for graphics tablets using the **Gotop T501 / 10moons / VINSA (`08f2:6811`)** chipset. Built and optimized for **Bazzite**, **Fedora**, **Arch Linux**, and other modern Linux distributions running Wayland or X11.

```text
ID 08f2:6811 Gotop Information Inc. [T501] Driver Inside Tablet
```

---

## 📌 What is it for?

On Linux, this tablet defaults to an "Android phone mode", restricting the active area to a tiny 3x6" rectangle in the upper corner.

This driver:
* **Unlocks the full 10x6" active area**.
* **Eliminates hover cursor flickering**: Stable cursor positioning near the surface (no rapid blinking or disappearing).
* **Adds natural drag-to-scroll**: Drag the pen on the tablet surface to scroll web pages, feeds, and PDF documents.
* **Hotplug support**: Starts automatically when plugged in and stops when disconnected.
* **Runs without root privileges**: Operates as a systemd user service via standard `uaccess` rules.

---

## ⚙️ How does it work?

The driver runs silently in the background and supports two switchable modes:

### 1. Pen Mode (Default)
* **Pen tip**: Drawing and handwriting with 6000 levels of pressure sensitivity (Krita, GIMP, Xournal++, etc.).
* **Cursor**: Absolute positioning across your displays.
* **Quick scroll shortcut**: Hold the lower barrel button on the pen and drag the tip to scroll at any time without switching modes.

### 2. Scroll Mode (Navigation)
Press physical button **"B"** (the 2nd button from the top on the tablet):
* **Drag pen up / down**: Smoothly scrolls the page under the cursor.
* **Quick tap**: Performs a standard left-click.
* Press **"B"** again to return to drawing mode.

---

## 📥 Download & Installation

The repository includes a ready-to-use precompiled binary and a one-step installer:

### Option A: Install from Git (Recommended)
```bash
git clone https://github.com/inaciogm/t501-tablet-linux-driver.git
cd t501-tablet-linux-driver
./install.sh
```

### Option B: Precompiled Tarball
* Download [`dist/tomate-mtm1106-driver-v1.1.0-linux-x86_64.tar.gz`](dist/tomate-mtm1106-driver-v1.1.0-linux-x86_64.tar.gz)
* Extract the archive and run `./install.sh`.

### Uninstallation
```bash
./uninstall.sh
```

---

## ⌨️ Physical Tablet Shortcuts

| Button | Key | Function |
| :---: | :---: | :--- |
| **1** | `Tab` | Tool switch / Tab |
| **2** | `B` | **Toggle Pen Mode / Scroll Mode** |
| **3** | `Alt` | Color picker / Alt shortcut |
| **4** | `Ctrl` | Ctrl modifier |
| **5** | `Page Up` | Scroll up |
| **6** | `Page Down` | Scroll down |
| **7** | `[` | Decrease brush size |
| **8** | `Ctrl + -` | Zoom out |
| **9** | `Ctrl + +` | Zoom in |
| **10** | `Esc` | Cancel / Escape |
| **11** | `Space` | Hand tool / Pan canvas |
| **12** | `]` | Increase brush size |

---

## 🛠️ Build from Source (Optional)

### On Bazzite / Fedora Silverblue (using Podman, no Rust needed on host):
```bash
podman run --rm \
  -v "$PWD:/work:Z" \
  -w /work/driver \
  registry.fedoraproject.org/fedora:41 \
  bash -lc 'dnf install -y cargo rustfmt clippy libusb1-devel && \
    cargo test --all-targets --locked && \
    cargo clippy --all-targets --locked -- -D warnings && \
    cargo build --release --locked'
```

### On Fedora / Arch / Ubuntu (native Rust):
```bash
cd driver
cargo test
cargo build --release
```

After building, run `./install.sh`.

---

## 📜 About the Fork

Forked from [feveal/Tablet-VINSA-1060-Plus-Linux-Driver](https://github.com/feveal/Tablet-VINSA-1060-Plus-Linux-Driver) and incorporating T501 protocol research from the [DIGImend](https://github.com/DIGImend/10moons-tools) project.

Rewritten for modern Wayland/systemd environments, resolving hover proximity noise, pressure hysteresis calibration, and adding native drag-to-scroll navigation. Licensed under [GPL-3.0](LICENSE).
