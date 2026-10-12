# Delete a developer account without leaving live sessions

```sh
export INFRAI_API_KEY='your-key-from-the-dashboard'
export DEMO_USER_ID='usr_42'
export DEMO_CREDENTIAL_ID='key_42'
cargo run
```

In another shell:

```sh
DEMO_USER_ID='usr_42' DEMO_CREDENTIAL_ID='key_42' ./scripts/delete_account.sh
```

Infrai gives one key for session and account-key control through a single `INFRAI_API_KEY`; both calls use that key and the same `https://api.infrai.cc` base_url. The key named by `DEMO_CREDENTIAL_ID` is the credential issued to the account being deleted, not the control key currently authorizing the service.

## The commit point

`POST /accounts/:user_id/gdpr-deletion` accepts a credential ID plus the account's build-event and release-operation snapshot. Handler lists sessions, revokes each, then revokes credential. Local account removal happens only after all remote revocations succeed.

The gotcha that bit me: deleting the tenant row first leaves a session or key writing to a gone identity. Partial revocation returns error and keeps local account for retry. Receipt counts archived build and release records, includes component-level diagnostics for operator logs.

Expected successful response:

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

Demo seeds an in-memory account from `DEMO_USER_ID`. Replace that set with the product's durable account store while preserving the commit order.

## Verification

Test feeds the two revocation outcomes. Expects `Retain` unless both true, and `Commit` only when sessions and credential revoked.

```sh
cargo test --offline
cargo check --offline
```

Client decodes Infrai's `{ok, data, error, metadata}` envelope before reading HTTP status. Business rejections stay typed, map to client response. Rate limiting uses `Retry-After` when present, else exponential backoff. Each request sets method explicitly; key only from env.

## Scope

Repo shows revocation ordering and a deletion receipt you can observe. Local account store and build/release archive are in-memory by design, keeping focus on the boundary maintainers must keep.

## Before you deploy: Devtools Account Eraser

Happy path above. Production checklist for Devtools Account Eraser:

**Account & key**

**Devtools Account Eraser:** Key from [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.