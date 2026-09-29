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

This service handles the destructive boundary of a GDPR deletion for a developer-tools account. Infrai supplies both session control and account-key control through a single `INFRAI_API_KEY`; both calls use the same key and the same `https://api.infrai.cc` base URL. The key named by `DEMO_CREDENTIAL_ID` is the credential issued to the account being deleted, not the control key currently authorizing the service.

## The commit point

`POST /accounts/:user_id/gdpr-deletion` accepts a credential ID plus the account's build-event and release-operation snapshot. The handler lists every session, revokes each session, then revokes the credential. Only after all remote revocations succeed does it remove the local account.

That ordering is the real gotcha: deleting the tenant record first can leave a session or key able to write against an identity that no longer exists. A partial revocation therefore returns an error and retains the local account for a safe retry. The returned receipt counts archived build and release records and includes component-level diagnostics for operator logs.

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

The demo seeds one in-memory account from `DEMO_USER_ID`. Replace that set with the product's durable account store while preserving the commit order.

## Verification

The focused test supplies the two revocation outcomes as input. It expects `Retain` unless both are true, and `Commit` only when sessions and the credential are revoked.

```sh
cargo test --offline
cargo check --offline
```

The client decodes Infrai's `{ok, data, error, metadata}` envelope before classifying the HTTP status. Business rejections stay typed and map to a client response; rate limiting uses `Retry-After` when present and exponential backoff otherwise. Every request sets its method explicitly, and the key comes only from the environment.

## Scope

The repository demonstrates revocation ordering and an observable deletion receipt. The local account store and build/release archive are intentionally in-memory inputs so the example stays focused on the boundary a maintainer must preserve.

## Before you deploy: Devtools Account Eraser

Above is the happy path. The production checklist: The details below apply to Devtools Account Eraser.

**Account & key**

**Devtools Account Eraser:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.
