//! Native GUI smoke test. Run on each desktop OS in a graphical session:
//! cargo run -p azdocs-desktop --example website_capture_smoke
//! Writes PNGs and a portable fixture database under output/website-smoke/.
use azdocs::{
    model::websites::{EndpointStatus, WebsiteEndpoint},
    store::Store,
};
use azdocs_desktop_lib::{
    WebsiteCaptureRequest,
    capture::{self, CaptureControl},
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::Arc,
};
use tauri::Manager as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = TcpListener::bind("127.0.0.1:0")?;
    let base = format!("http://{}", server.local_addr()?);
    std::thread::spawn(move || {
        for mut connection in server.incoming().flatten() {
            std::thread::spawn(move || {
                let mut buffer = [0; 4096];
                let n = connection.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..n]);
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                if path == "/redirect" {
                    let _ = connection.write_all(b"HTTP/1.1 302 Found\r\nLocation: /\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    return;
                }
                if path == "/asset.svg" {
                    std::thread::sleep(std::time::Duration::from_millis(700));
                    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='40'><rect width='100' height='40' fill='#cd8038'/></svg>";
                    let _ = connection.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: image/svg+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{svg}", svg.len()).as_bytes());
                    return;
                }
                if path == "/never" {
                    std::thread::sleep(std::time::Duration::from_secs(35));
                    return;
                }
                let code = if path == "/error" {
                    "503 Service Unavailable"
                } else {
                    "200 OK"
                };
                let body = format!(
                    r#"<!doctype html><html><head><meta charset="utf-8"><style>body{{margin:0;background:#eef3f5;color:#172936;font:22px system-ui}}header{{background:#172936;color:white;padding:35px 70px}}main{{padding:65px 70px}}h1{{font-size:64px;margin:25px 0}}.status{{background:#666666;color:white;padding:25px;max-width:700px}}.grid{{display:flex;gap:25px;margin-top:40px}}.grid p{{background:white;padding:35px;flex:1}}</style></head><body><header>AZDOCS NATIVE CAPTURE FIXTURE</header><main><div id="paint" style="position:absolute;left:1000px;top:700px;width:100px;height:50px;background:#666"></div><p>Local test website · {code}</p><img src="/asset.svg" width="100" height="40" style="position:absolute;left:1250px;top:160px"><h1 id="heading">Waiting for JavaScript</h1><div class="status" id="status">Loading application</div><div class="grid"><p>Static Web App</p><p>App Service</p><p>Front Door</p></div></main><script>const cleanSession=!document.cookie; document.cookie='fixture=seen;path=/';setTimeout(()=>{{document.getElementById('paint').style.background=document.getElementById('status').style.background=cleanSession?'#18755a':'#cc0000';document.getElementById('heading').textContent='Website evidence, saved.';document.getElementById('status').textContent='JavaScript rendered · no browser driver';}},600);</script></body></html>"#
                );
                let response = format!(
                    "HTTP/1.1 {code}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = connection.write_all(response.as_bytes());
            });
        }
    });
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../output/website-smoke");
    std::fs::create_dir_all(&output)?;
    let path = output.join("fixture.db");
    let store = Store::open(&path)?;
    let snapshot = store.create_snapshot("smoke", None)?;
    store.set_snapshot_status(&snapshot.id, azdocs::model::SnapshotStatus::Complete)?;
    let paths = ["/", "/redirect", "/error", "/never"];
    let endpoints: Vec<_> = paths
        .iter()
        .map(|suffix| WebsiteEndpoint {
            resource_id: "/smoke/site".into(),
            resource_name: "Native capture fixture".into(),
            source: (*suffix).into(),
            hostname: Some("127.0.0.1".into()),
            url: Some(format!("{base}{suffix}")),
            status: EndpointStatus::Ready,
        })
        .collect();
    store.save_website_inventory(&snapshot.id, &endpoints, &[])?;
    drop(store);
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.truncate(1);
    context.config_mut().app.windows[0].url = tauri::WebviewUrl::External("about:blank".parse()?);
    let control = Arc::new(CaptureControl::default());
    let app = tauri::Builder::default().setup(move |app| {
        let app=app.handle().clone();
        tauri::async_runtime::spawn(async move {
            let run = async {
                let lease=control.begin().map_err(|e|e.to_string())?;
                let request=WebsiteCaptureRequest {snapshot_id:snapshot.id.clone(),urls:vec![],retry_only:false};
                let result=capture::run(&app,&path,request,&lease,|p| { assert_eq!(app.windows().len(), 1, "Capture opened a popup"); println!("Capture {}/{} preview={}",p.completed,p.total,p.preview_image.is_some()); }).await.map_err(|e|e.to_string())?;
                let store=Store::open(&path).map_err(|e|e.to_string())?;
                for (i,c) in store.website_captures(&snapshot.id,true).map_err(|e|e.to_string())?.iter().enumerate() {
                    if !c.png.is_empty() {
                        let image = image::load_from_memory(&c.png).map_err(|e|e.to_string())?.to_rgb8();
                        if image.dimensions() != (1440,900) || image.get_pixel(1020,720).0 != [24,117,90] || image.get_pixel(1270,180).0 != [205,128,56] { return Err("Painted content, delayed asset, JavaScript or session isolation check failed".into()); }
                        std::fs::write(output.join(format!("capture-{i}.png")),&c.png).map_err(|e|e.to_string())?; }
                    println!("{} {:?} final={:?} error={:?}",c.url,c.status,c.final_url,c.error);
                }
                if result.captured!=3 || result.failed!=1 { return Err(format!("Unexpected batch result: {result:?}")); }
                if app.webviews().keys().any(|label| label.starts_with("website-capture-")) { return Err("Capture window leaked".into()); }
                drop(lease);
                let lease = control.begin().map_err(|e|e.to_string())?;
                let cancel_control = Arc::clone(&control);
                tauri::async_runtime::spawn(async move { tokio::time::sleep(std::time::Duration::from_millis(700)).await; cancel_control.cancel(); });
                let result = capture::run(&app, &path, WebsiteCaptureRequest { snapshot_id: snapshot.id.clone(), urls: vec![format!("{base}/never")], retry_only: false }, &lease, |_| {}).await.map_err(|e|e.to_string())?;
                if !result.cancelled || app.webviews().keys().any(|label| label.starts_with("website-capture-")) { return Err("Cancellation leaked a capture window".into()); }
                println!("Native capture smoke test passed. Output: {}",output.display());
                Ok::<(),String>(())
            }.await;
            if let Err(error)=run { eprintln!("{error}");app.exit(1); } else { app.exit(0); }
        });
        Ok(())
    }).build(context)?;
    app.run(|_, event| {
        if let tauri::RunEvent::ExitRequested {
            code: None, api, ..
        } = event
        {
            api.prevent_exit()
        }
    });
    Ok(())
}
