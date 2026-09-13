# Contributing to azdocs

Start with the [development guide](docs/development/README.md) for setup and
checks, then the [contribution recipes](docs/development/contributing.md) for
queries, reports, diagrams, desktop work and schema changes.

Use [GitHub issues](https://github.com/russmckendrick/azdocs/issues) for bugs and
feature requests. Include the azdocs version or commit, operating system,
command or screen, expected behaviour and a minimal, sanitised reproduction.
Use synthetic Azure IDs and resource names. Do not upload credentials, real
estate databases, configuration backups or unredacted reports.

For vulnerabilities, follow [SECURITY.md](SECURITY.md) instead of opening a
public issue.

Before a pull request, run the checks appropriate to the change and explain
what you verified. Keep generated wire contracts and reviewed golden files
current. Documentation should link to the canonical page rather than repeat
changing counts, platform requirements or release claims.

Contributions to azdocs code and documentation are under the [MIT licence](LICENSE).
Preserve upstream notices when adapting queries, adding assets or vendoring
code; see [Third-party notices](THIRD_PARTY_NOTICES.md).
