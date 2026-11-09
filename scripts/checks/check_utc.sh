#!/bin/sh
# Lists the commits in a range whose author or committer time isn't recorded in UTC
# (+0000), and fails if there are any. Commit times carry the committer's time zone,
# and this repo keeps them all in UTC.
#   sh scripts/checks/check_utc.sh <rev-list args>     e.g.  origin/main..HEAD
# Used by .githooks/pre-push.
[ $# -gt 0 ] || { echo "usage: sh scripts/checks/check_utc.sh <range>" >&2; exit 2; }
bad=$(git log --format='%h %ad|%cd %s' --date=raw "$@" |
      grep -v '^[0-9a-f]* [0-9]* +0000|[0-9]* +0000 ')
[ -z "$bad" ] && exit 0
echo "Commits not stamped in UTC (+0000):" >&2
echo "$bad" | sed 's/^/  /' >&2
echo "Re-stamp them in UTC before pushing (the post-commit hook does this for new commits)." >&2
exit 1
