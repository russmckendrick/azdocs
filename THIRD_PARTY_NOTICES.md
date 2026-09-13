# Third-party notices

The [MIT licence](LICENSE) covers azdocs's own code and documentation. It does
not replace the terms of bundled fonts, Microsoft artwork, adapted upstream
queries or software dependencies.

## Fonts

IBM Plex Serif, Sans and Mono are included under the SIL Open Font License 1.1.
Copyright © 2017 IBM Corp., with Reserved Font Name “Plex”. Font files are in
`data/fonts/` and `desktop/src/assets/fonts/`; the complete notice and licence
are in [data/fonts/OFL.txt](data/fonts/OFL.txt). Upstream:
[IBM Plex](https://github.com/IBM/plex).

The native PDF renderer also loads Libertinus Serif and DejaVu Sans Mono as
fallbacks from `typst-assets`. Its bundled font copyright and licence notices
are retained in [docs/licenses/typst-assets-NOTICE.txt](docs/licenses/typst-assets-NOTICE.txt),
copied from `typst-assets` 0.13.1. Refresh this notice when that dependency changes.
Installed Charter, Arial and Courier New files are not distributed in this
repository; PDF may embed locally installed faces, while Word names them.

## Microsoft Azure architecture icons

`data/icons/` and the desktop's derived service-icon assets contain Microsoft
Azure architecture artwork. Microsoft retains its rights in these icons; they
are not relicensed under MIT. They are used to identify Azure services in
estate diagrams and documentation. Microsoft's
[icon terms and usage guidance](https://learn.microsoft.com/en-us/azure/architecture/icons/)
permit architectural diagrams, training materials and documentation, and limit
copying, distribution and display to those uses unless Microsoft grants further
permission. Keep the artwork's shape and proportions and do not use it to
represent an unrelated product or service.

The azdocs application mark is separate, original project artwork. azdocs is
not affiliated with or endorsed by Microsoft. Microsoft and Azure are trademarks
of Microsoft Corporation.

## Adapted Microsoft queries

Selected queries adapt examples and guidance from the
[Microsoft FinOps Toolkit](https://github.com/microsoft/finops-toolkit) and
[Azure Proactive Resiliency Library](https://github.com/Azure/Azure-Proactive-Resiliency-Library-v2).
Both publish their code under MIT, copyright Microsoft Corporation. The retained
notice is [docs/licenses/Microsoft-MIT.txt](docs/licenses/Microsoft-MIT.txt).
Query comments and the [query reference](https://github.com/russmckendrick/azdocs/blob/main/docs/reference/queries.md) identify
sources and explain adaptations. Links to Microsoft Learn describe API behaviour
and evidence interpretation; they do not imply ownership of that documentation.

## Software dependencies

Rust dependencies are locked in `Cargo.lock`; desktop JavaScript dependencies
are locked in `desktop/pnpm-lock.yaml`. Each dependency retains its own licence.
The release workflow generates `dependency-licenses.html` using cargo-about
from the locked CLI graph, including the supported platforms, and packages it
with these notices and the font licences. See
[Releasing](https://github.com/russmckendrick/azdocs/blob/main/docs/development/releasing.md)
for generation and review commands. The generated list includes source links;
for MPL-2.0 dependencies, the linked crate archives contain their corresponding
source files.

Published desktop bundles embed this project notice, the MIT licence and the
retained font, artwork and query licence material. Rust and JavaScript
dependencies remain identified by `Cargo.lock` and `desktop/pnpm-lock.yaml`;
each dependency retains its own licence.
