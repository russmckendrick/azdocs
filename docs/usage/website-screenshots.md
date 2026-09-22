# Website screenshots

Desktop collection automatically captures the initial page of websites associated
with Static Web Apps, App Service, Application Gateways and Azure Front Door.
The images are saved in the selected SQLite snapshot. Capture progress and
failures are separate from Azure's complete/partial collection outcome.

## Capture and review

Open a resource's **Website screenshots** section to preview saved images.
**View full image** opens a keyboard-accessible image viewer; Escape closes it
and returns focus to the preview. **Save PNG** writes the original 1440 × 900
image using the system save dialog. **Refresh screenshot** replaces an image
only after a successful capture.

The **Collect snapshot** button opens a dialog with a new Azure collection action.
Its **Website screenshots** section offers **Capture all websites**, **Retry missing or failed**
and **Cancel screenshots** for the selected snapshot. You can close the dialog
while work continues and reopen it to check progress. Requests are sequential, with a 30-second deadline per URL,
document readiness and a two-second settling period. Live feedback shows completed queries, stored rows, collection stages, elapsed time,
and the current URL with saved/failed/remaining counts. The completion summary
stays in the dialog until the next collection. The capture time can be later
than the Azure audit time. A failed or cancelled refresh retains the previous PNG,
its original capture time and final URL, alongside the latest attempt outcome.
An interrupted batch is marked when the database next opens.

Capture uses the machine's network access, including internal sites reachable
through a VPN. Sessions start without existing browser cookies or sign-in state.
No sign-in automation or Azure credentials are supplied to websites. A rendered
login page, access-denied page or HTTP error page is useful evidence and is saved;
DNS, certificate, navigation and timeout failures are recorded separately.
Downloads, popups, device permission requests and non-HTTP(S) navigation are denied.

macOS uses WebKit, Windows uses WebView2, and Linux uses WebKitGTK. This does not
install Chrome or a browser driver. WebView2 uses Chromium as part of Windows'
native webview runtime. Capture previews appear inside the collection panel,
without opening another browser window. A fixed-size isolated child view renders
the original 1440 × 900 image outside the visible workspace and sends image
previews to the panel. The view is destroyed after each attempt, including
cancellation and timeouts.

## Which endpoints are included

| Service | Evidence |
|---|---|
| Static Web Apps | Default hostname and stored custom domains |
| App Service and deployment slots | Default hostname and bound hostnames; SCM/Kudu excluded |
| Application Gateway | Listener hostnames, explicit HTTP/HTTPS protocol and frontend port |
| Front Door Classic | Frontend hostnames and enabled routing-rule protocols |
| Front Door Standard/Premium | Default endpoint hostnames, custom domains and route associations |

Captures use `/`, prefer HTTPS, and use HTTP for explicitly HTTP-only listeners
or routes. Duplicate URLs share one PNG per snapshot while keeping all resource
associations. Wildcard/unnamed listeners, stopped or disabled endpoints, invalid
hostnames and missing evidence remain visible as skipped records.

Desktop collection supplements Front Door Resource Graph properties with
paginated read-only management API requests for custom domains and routes. It
reuses the configured token provider and Reader access. Responses and lookup
failures are stored separately from ARG results. Older snapshots can capture
hostnames already present in their evidence; collecting a fresh snapshot is
necessary to discover missing Front Door domain evidence.

## Offline exports

HTML embeds PNG bytes in the self-contained report. The HTML site writes local
`websites/*.png` assets and associates galleries with resource-group pages.
PDF and DOCX screenshots appear in the optional **technical reference**
(`--include-reference`), sized to the page column with their original aspect ratio.
Each resource's saved images are followed by a separate table for each website containing
the requested links, status, capture time and redirect destinations. Links show the full stored URL with an external-link icon and wrap within
the table cells without truncation. Uncaptured websites and failed refreshes appear in their own tables.
The main assessments remain focused on assessment content.

Exporters read stored evidence only; they never contact Azure or websites.
The CLI has no capture command, but it can export screenshots saved by the desktop.
The browser preview uses a bundled local fixture and never visits sample websites.
