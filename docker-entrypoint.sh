#!/bin/sh

set -e

# Default to root, so old installations won't break
export PUID=${PUID:-0}
export PGID=${PGID:-0}

# Check if the specified group with PGID exists, if not, create it.
if ! getent group "$PGID" >/dev/null; then
  groupadd -g "$PGID" appgroup
fi
# Create user if it doesn't exist.
if ! getent passwd "$PUID" >/dev/null; then
  useradd --create-home --shell /bin/sh --uid "$PUID" --gid "$PGID" appuser
fi

# Set privileges for /app but only if pid 1 user is root and we are dropping privileges.
# If container is run as an unprivileged user, it means owner already handled ownership setup on their own.
# Running chown in that case (as non-root) will cause error
[ "$(id -u)" = "0" ] && [ "${PUID}" != "0" ] && chown -R ${PUID}:${PGID} /app && chown -R ${PUID}:${PGID} /biliup

# Drop privileges (when asked to) if root, otherwise run as current user
if [ "$(id -u)" = "0" ] && [ "${PUID}" != "0" ]; then
  exec setpriv --reuid=$PUID --regid=$PGID --init-groups "$@"
else
  exec "$@"
fi
