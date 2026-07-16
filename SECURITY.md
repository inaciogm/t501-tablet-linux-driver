# Security policy

This project controls a USB device and creates virtual input devices. Treat any
change to USB matching, feature reports, `uinput`, dependencies, installation,
or service permissions as security-sensitive.

Before installing, review the source, build from the locked dependency graph,
and compare the requested permissions with `AUDIT.md`. Never replace the
included active-user ACL with `MODE="0666"` rules.

For a suspected vulnerability, use the repository's private GitHub security
advisory feature when available. Include the affected commit, reproduction
steps, and whether physical access to a matching USB device is required. Avoid
publishing exploit details before a fix is available.
