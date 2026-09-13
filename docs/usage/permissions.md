# Permission diagnostics

**Test connection**, `azdocs check`, collection preflight and live query
preflight share one Rust diagnostic. Tests authenticate the selected identity,
list visible subscriptions, identify configured subscriptions that are not
visible, inspect Azure RBAC evidence and record the check time.

| Verdict | Meaning |
|---|---|
| Read-only grants in checked scopes | All inspected permission blocks contain only read grants after their own exclusions, and coverage completed. |
| Broader grants found | At least one inspected block grants a non-read operation. This warning remains visible even if another scope is unverifiable. |
| Unable to verify | Identity, assignments, definitions, conditions or permission patterns prevent a complete read-only determination. |

These are advisory warnings. Authentication failures stop the operation. A
warning does not stop an authorised collection or live query, and the check
never attempts writes. Desktop results show roles, affected scopes, visible and
inaccessible subscriptions, coverage limitations and time. Collection feedback
retains its permission warning. A draft test belongs only to that draft;
identity, credential or scope edits invalidate it. Live execution checks again.

## Evidence and limits

Subscription discovery uses Azure Resource Graph. ARM assignment requests use
`assignedTo('<principal-object-id>')`, including supported transitive group
expansion and assignments above, at and below the subscription scope. The
principal object ID comes from the acquired token; an unavailable ID produces
an unverifiable result. No Microsoft Graph permission is requested.

Role definitions determine safety, never role names. `Actions` minus
`NotActions`, and `DataActions` minus `NotDataActions`, are evaluated within each
permission block; grants from different blocks and roles are then combined.
An exclusion in one block cannot cancel another block's grant. Unsupported
wildcard subtraction or conditional grants cannot yield a reassuring verdict.

The checker follows continuation links only on the ARM origin, detects cycles,
retries throttling/server errors up to three attempts and bounds requests and
the assignment/definition phase. Partial page evidence is retained, with a
coverage warning. Missing permissions to inspect RBAC produce **Unable to
verify**, not a request for more privileged credentials.

This is evidence about active Azure RBAC grants in the checked scopes. It is
not tenant-wide effective-access certification, and does not certify service
ACLs, inactive eligible roles or access through other authentication systems.

API semantics: [assignment queries](https://learn.microsoft.com/en-us/azure/role-based-access-control/role-assignments-list-rest),
[role definitions and permission exclusions](https://learn.microsoft.com/en-us/azure/role-based-access-control/role-definitions).

Next: [Collecting](collecting.md)
