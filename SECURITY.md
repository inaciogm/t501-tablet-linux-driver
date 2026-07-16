# Security

This driver needs USB and `uinput` access to work. Read the source and the
included `udev` rule before installing it. Avoid replacing the rule with broad
`MODE="0666"` permissions.

The checks I made are listed in [AUDIT.md](AUDIT.md). If you find a security
problem, please report it privately through GitHub when possible instead of
posting exploit details in a public issue.
