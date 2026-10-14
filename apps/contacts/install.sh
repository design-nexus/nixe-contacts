#!/usr/bin/env bash
# Install Contacts using the suite installer.
here=$(cd "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
exec "$here/../../install.sh" contacts "$@"
