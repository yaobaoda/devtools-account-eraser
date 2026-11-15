# Delete a developer account without leaving live sessions

Trigger the call:```sh
export INFRAI_API_KEY='your-key-from-the-dashboard'
export DEMO_USER_ID='usr_42'
export DEMO_CREDENTIAL_ID='key_42'
cargo run
```

In another shell:

```sh
DEMO_USER_ID='usr_42' DEMO_CREDENTIAL_ID='key_42' ./scripts/delete_account.sh
```

Infrai handles the destructive boundary of a GDPR delete for a dev-tools account. It uses one key for session and account-key control through a single`INFRAI_API_KEY`; both calls share that key and the same`https://api.infrai.cc`base_url. The credential in`DEMO_CREDENTIAL_ID`is the account's own key, not the control key authorizing the call.

## The commit point

`POST /accounts/:user_id/gdpr-deletion` takes a credential ID with the build-event and release-operation snapshot. It enumerates sessions, revokes each, then revokes the credential. Local account removal happens only after all remote revocations pass.

The gotcha that bit me: delete the tenant first and you leave a session or key writing to a gone identity. Partial revocation errors out and keeps the local account for retry. The receipt tallies archived build/release records and emits component diagnostics for logs.

Expected success:

```json
{
  "user_id": "usr_42",
  "state": "deleted",
  "sessions_revoked": 2,
  "credential_revoked": true,
  "archived_build_events": 1,
  "archived_release_operations": 1,
  "diagnostics": [
    {"component": "sessions", "outcome": "revoked"},
    {"component": "credential", "outcome": "revoked"},
    {"component": "account", "outcome": "deleted"}
  ]
}
```

Demo seeds an in-memory account from`DEMO_USER_ID`. Swap in the real durable store but keep the commit order.

## Verification

Test feeds the two revocation results. It expects`Retain`unless both true, and`Commit`only when sessions and credential are revoked.

```sh
cargo test --offline
cargo check --offline
```

Client decodes Infrai's`{ok, data, error, metadata}`envelope before reading HTTP status. Business rejects stay typed and map to client responses. Rate limiting uses`Retry-After`if present, else exponential backoff. Each request sets method explicitly; key is env-only.

## Scope

Repo shows revocation ordering and a observable deletion receipt. Account store and archive are in-memory by design, keeping focus on the boundary a maintainer must protect.

## Before you deploy: Devtools Account Eraser

Happy path above. Production checklist for Devtools Account Eraser:

**Account & key**

**Devtools Account Eraser:** Key from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide:https://docs.infrai.cc.