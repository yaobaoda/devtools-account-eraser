#!/bin/sh
set -eu

user_id="${DEMO_USER_ID:-user_demo}"
credential_id="${DEMO_CREDENTIAL_ID:?set DEMO_CREDENTIAL_ID to the account credential id}"

curl --fail-with-body --request POST \
  "http://127.0.0.1:3000/accounts/${user_id}/gdpr-deletion" \
  --header 'content-type: application/json' \
  --data "{\"credential_id\":\"${credential_id}\",\"build_events\":[{\"build_id\":\"build-1042\",\"status\":\"succeeded\"}],\"release_operations\":[{\"release_id\":\"release-88\",\"environment\":\"production\"}]}"

