#!/bin/sh
set -eu

# Needs root for a complete installation check. systemd-analyze verify itself
# is safe to run without root and validates the unit syntax and dependencies.
unit=${1:-packaging/focus-on-work.service}
systemd-analyze verify "$unit"
echo "Unit verification passed. A root/systemd VM run is still needed to verify"
echo "the protected /etc write and socket lifecycle."
