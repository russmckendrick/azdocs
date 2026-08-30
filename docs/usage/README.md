# Usage

The five-minute path from nothing to a documented estate:

```mermaid
flowchart LR
    sp[Create read-only<br/>service principal] --> init[azdocs init]
    init --> check[azdocs check]
    check --> collect[azdocs collect]
    collect --> out{Explore}
    out --> report[azdocs report --format all]
    out --> diagram[azdocs diagram --type network]
    out --> browse[azdocs browse]
    out --> desktop["Desktop explorer (Tauri app)"]
```

1. [Install](installation.md) the binary
2. [Configure](configuration.md) a read-only service principal
3. [Collect](collecting.md) a snapshot
4. Explore in the [desktop app](desktop.md), export [reports](reports.md) and
   [diagrams](diagrams.md), or [browse](tui.md) interactively
5. [Diff snapshots](snapshots.md) over time, extend the [query pack](queries.md), wire it into [CI](ci.md)

Stuck? See [troubleshooting](troubleshooting.md).
