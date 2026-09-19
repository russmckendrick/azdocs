# Frequently asked questions

## Does azdocs change anything in Azure?

No. Collection uses Azure Resource Graph and a few read-only ARM calls with a
Reader identity; there is no write path. See
[permission diagnostics](permissions.md).

## Do reports need network access?

No. Every report, diagram and the desktop explorer read the local SQLite
snapshot only. Collection and website capture are the only steps that go
online. See [reports](reports.md).

## Why does `latest` skip a snapshot I can see in the list?

`latest` and "the previous snapshot" resolve only to `complete` or `partial`
snapshots. A `failed`, `cancelled` or still-`running` snapshot has to be named
by id. See [snapshots](snapshots.md).

## The report says "Showing 20 of 300 rows". Where is the rest?

Printed formats stop tables at `[report] max_evidence_rows` and say so; the
CSV and XLSX exports and the snapshot database hold everything. See
[reports › honest caps](reports.md#honest-caps).

## A finding names a resource that is not in the inventory. Is that a bug?

No. Some checks report on subscriptions, assessments or resources outside the
collected scope; the report keeps them as unresolved occurrences rather than
dropping evidence. See [collecting](collecting.md).

## The diff calls every resource "changed". Why?

Azure rewrites some properties on every read. The built-in noise list in
`data/diff_ignore.toml` covers the usual ones; add estate-specific paths in
`<config dir>/azdocs/diff_ignore.toml`. See [snapshots](snapshots.md#diffing-estates-over-time).

## Can I run it against Azure Government or Azure China?

Yes: set `cloud = "usgov"` or `cloud = "china"` in the configuration or a
tenant profile. See [configuration](configuration.md#sovereign-clouds).

## Where does my client secret go?

Into the OS credential store, or nowhere at all when a profile references an
environment variable. It is never written to the configuration file by
`init` or the desktop. See [configuration](configuration.md) and
[data on disk](data-and-uninstall.md).

## Windows says "Windows protected your PC" when I run the installer

The installer is unsigned unless the release was built with Trusted Signing
credentials. Verify the checksum or attestation and choose *More info › Run
anyway*. See [installation](installation.md#windows-smartscreen).

## How do I remove everything?

See [data on disk and uninstalling](data-and-uninstall.md).

Next: [Troubleshooting](troubleshooting.md)
