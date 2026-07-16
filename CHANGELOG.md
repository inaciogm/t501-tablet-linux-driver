# Changelog

## Unreleased — Tomate MTM-1106 / Bazzite reliability branch

- Restore the complete four-report DIGImend initialization sequence.
- Use a deterministic send-close-reopen-reset-claim workflow for full-area
  64-byte reports.
- Default to absolute tablet mode and remove drawing-mode smoothing.
- Correct virtual pen touch, proximity, axis, and pressure declarations.
- Initialize once per connection and exit on USB removal.
- Add event-driven Bazzite/KDE packaging with exact-device `uaccess` rules.
- Remove the obsolete desktop launcher and unsafe world-writable instructions.
- Add regression tests for the full USB initialization sequence and raw pen
  report decoding.
- Emit each pen sample as one atomic evdev batch, keeping X, Y, pressure,
  proximity, touch, and pen buttons under the same `SYN_REPORT`.
- Remove the obsolete launcher images and avoid unnecessary startup clones.
