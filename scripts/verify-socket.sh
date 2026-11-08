#!/bin/sh
set -eu

# Needs root and two disposable unprivileged users. Run in a disposable VM
# after installing the package; this starts a root daemon and terminates
# matching processes.
user=${1:?usage: $0 UNPRIVILEGED_USER}
other_user=${2:?usage: $0 IN_GROUP_USER NOT_IN_GROUP_USER}
echo "Manual VM check:"
echo "  sudo ./install.sh"
echo "  sudo usermod -aG focus-on-work $user"
echo "  # Log out and back in, then:"
echo "  focus-on-work status"
echo "  sudo -u $other_user focus-on-work status  # must be denied"
echo "  stat -c '%U:%G %a %n' /run/focus-on-work /run/focus-on-work/focus-on-work.sock"
echo "Expected: root:focus-on-work 770 for the directory and"
echo "root:focus-on-work 660 for the socket."
echo "This script needs manual run in a VM; it does not modify the real hosts file."
