# Security

## Reporting a vulnerability

Email [github@mckendrick.email](mailto:github@mckendrick.email) with the subject
“azdocs security report”. This is the maintainer's public GitHub contact.
Describe the affected version or commit, platform, impact and reproduction
steps. Use synthetic data wherever possible and arrange a private transfer
before sending sensitive evidence. Do not open a public issue containing an
exploit, credential, database or customer estate details.

There is no guaranteed response time or commercial support commitment.
Fixes are developed on `main` and shipped in the next release.

## Supported versions

| Version | Supported |
|---|---|
| The latest release on GitHub Releases and the Homebrew tap | Yes: security fixes ship as a new patch or minor release |
| Older releases | No: upgrade to the latest release |
| `main` between releases | Best effort; it carries fixes before they are tagged |

Dependencies are checked on every push by `cargo deny` (advisories, licences,
duplicate versions and registries) and `pnpm audit`, and CLI archives carry a
signed build-provenance attestation and an SPDX SBOM; see
[releasing](docs/development/releasing.md#supply-chain).

## Security boundaries

- Azure collection and preflight use read-only requests. The recommended
  identity has Reader access to the scopes being audited. Advisory RBAC
  diagnostics cannot certify all effective access held by a credential.
- Named profiles store secrets in the OS credential store or reference a
  process environment variable. Existing secrets and OAuth tokens are not
  returned to the desktop frontend. Legacy configuration files and migration
  backups can contain plaintext credentials.
- SQLite snapshots contain estate configuration, identifiers, findings and
  potentially sensitive website images. They are not encrypted by azdocs.
  Protect the database, its WAL/SHM files, backups and exported reports using
  operating-system permissions and your organisation's storage controls.
- Desktop website capture visits discovered URLs using the machine's network
  access, including VPN access. Capture uses a separate session without your
  browser cookies or Azure credentials. Saved images may contain login or
  error pages. Exporting or viewing stored evidence does not visit the sites.

See [configuration](docs/usage/configuration.md),
[permission diagnostics](docs/usage/permissions.md) and
[website screenshots](docs/usage/website-screenshots.md) for the implementation's
scope and limitations. Read-only Azure access does not make collected data safe
to publish.
