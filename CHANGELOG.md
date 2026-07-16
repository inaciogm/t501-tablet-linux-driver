# Changes

## Tomate MTM-1106 / Bazzite version

- Added the complete initialization needed for the full tablet area.
- Made the driver start on connection and stop on removal.
- Fixed pressure, proximity, touch, axes, and pen button events.
- Fixed stair-step diagonal lines by sending each pen sample in one batch.
- Added the Bazzite/KDE systemd service and exact-device `udev` rule.
- Removed the old launcher, unused images, and unsafe `0666` instructions.
- Added basic tests for initialization and pen report decoding.
