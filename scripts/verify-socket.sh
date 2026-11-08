#!/bin/sh
set -eu

# Needs root and two disposable unprivileged users. Run in a disposable VM
# after installing the package; the daemon may edit /etc/hosts and terminate
# matching processes.
set -eu

in_group_user=${1:?usage: $0 IN_GROUP_USER NOT_IN_GROUP_USER}
not_in_group_user=${2:?usage: $0 IN_GROUP_USER NOT_IN_GROUP_USER}

if [ "$(id -u)" -ne 0 ]; then
    echo "run this script as root" >&2
    exit 1
fi
if ! id -nG "$in_group_user" | tr ' ' '\n' | grep -qx focus-on-work; then
    echo "$in_group_user is not currently in focus-on-work; log in again first" >&2
    exit 1
fi
if id -nG "$not_in_group_user" | tr ' ' '\n' | grep -qx focus-on-work; then
    echo "$not_in_group_user is unexpectedly in focus-on-work" >&2
    exit 1
fi

systemctl is-active --quiet focus-on-work.service
stat -c '%U:%G %a %n' /run/focus-on-work /run/focus-on-work/focus-on-work.sock
directory_mode=$(stat -c '%a' /run/focus-on-work)
socket_mode=$(stat -c '%a' /run/focus-on-work/focus-on-work.sock)
[ "$(stat -c '%U:%G' /run/focus-on-work)" = "root:focus-on-work" ]
[ "$(stat -c '%U:%G' /run/focus-on-work/focus-on-work.sock)" = "root:focus-on-work" ]
[ "$directory_mode" = 770 ]
[ "$socket_mode" = 660 ]

echo "Checking status as group member: $in_group_user"
runuser -u "$in_group_user" -- focus-on-work status
echo "Checking denial for non-member: $not_in_group_user"
if runuser -u "$not_in_group_user" -- focus-on-work status; then
    echo "non-member unexpectedly accessed the daemon socket" >&2
    exit 1
fi
echo "Socket permission checks passed."
