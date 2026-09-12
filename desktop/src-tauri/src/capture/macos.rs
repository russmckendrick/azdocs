use super::{ImageSender, NavigationError};
use block2::{DynBlock, RcBlock};
use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
use objc2_foundation::{
    MainThreadMarker, NSDictionary, NSError, NSObject, NSObjectProtocol, NSString,
};
use objc2_web_kit::*;
use std::{cell::RefCell, collections::BTreeMap, sync::Mutex};

pub const RENDERER: &str = "WebKit";

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = NavigationError]
    struct CaptureDelegate;

    unsafe impl NSObjectProtocol for CaptureDelegate {}
    unsafe impl WKNavigationDelegate for CaptureDelegate {
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn policy(
            &self,
            _view: &WKWebView,
            action: &WKNavigationAction,
            handler: &DynBlock<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            let url = unsafe { action.request().URL() }
                .and_then(|u| u.absoluteString())
                .map(|u| u.to_string());
            let allowed = url.as_deref().is_some_and(super::super::allowed_navigation);
            handler.call((if allowed {
                WKNavigationActionPolicy::Allow
            } else {
                WKNavigationActionPolicy::Cancel
            },));
        }
        #[unsafe(method(webView:decidePolicyForNavigationResponse:decisionHandler:))]
        fn response(
            &self,
            _view: &WKWebView,
            response: &WKNavigationResponse,
            handler: &DynBlock<dyn Fn(WKNavigationResponsePolicy)>,
        ) {
            let allowed = unsafe { response.canShowMIMEType() };
            if !allowed && let Ok(mut error) = self.ivars().lock() {
                *error = Some(super::failure("Response is a download, not a webpage"));
            }
            handler.call((if allowed {
                WKNavigationResponsePolicy::Allow
            } else {
                WKNavigationResponsePolicy::Cancel
            },));
        }
        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn provisional_failure(
            &self,
            _view: &WKWebView,
            _navigation: Option<&WKNavigation>,
            error: &NSError,
        ) {
            if let Ok(mut state) = self.ivars().lock() {
                *state = Some(error.localizedDescription().to_string());
            }
        }
        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn failure(&self, _view: &WKWebView, _navigation: Option<&WKNavigation>, error: &NSError) {
            if let Ok(mut state) = self.ivars().lock() {
                *state = Some(error.localizedDescription().to_string());
            }
        }
        #[unsafe(method(webViewWebContentProcessDidTerminate:))]
        fn terminated(&self, _view: &WKWebView) {
            if let Ok(mut state) = self.ivars().lock() {
                *state = Some(super::failure("Web content process terminated"));
            }
        }
    }
    unsafe impl WKUIDelegate for CaptureDelegate {
        #[unsafe(method(webView:requestMediaCapturePermissionForOrigin:initiatedByFrame:type:decisionHandler:))]
        fn media(
            &self,
            _view: &WKWebView,
            _origin: &WKSecurityOrigin,
            _frame: &WKFrameInfo,
            _kind: WKMediaCaptureType,
            handler: &DynBlock<dyn Fn(WKPermissionDecision)>,
        ) {
            handler.call((WKPermissionDecision::Deny,));
        }
        #[unsafe(method(webView:requestDeviceOrientationAndMotionPermissionForOrigin:initiatedByFrame:decisionHandler:))]
        fn motion(
            &self,
            _view: &WKWebView,
            _origin: &WKSecurityOrigin,
            _frame: &WKFrameInfo,
            handler: &DynBlock<dyn Fn(WKPermissionDecision)>,
        ) {
            handler.call((WKPermissionDecision::Deny,));
        }
    }
);

// WK delegates are weak references; retain them until their view is destroyed.
thread_local! { static DELEGATES: RefCell<BTreeMap<String, Retained<CaptureDelegate>>> = const { RefCell::new(BTreeMap::new()) }; }

pub fn configure(view: tauri::webview::PlatformWebview, error: NavigationError, label: String) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let delegate: Retained<CaptureDelegate> =
        unsafe { msg_send![super(CaptureDelegate::alloc(mtm).set_ivars(error)), init] };
    // Tauri dispatches with_webview on the UI thread and owns this WKWebView.
    unsafe {
        let webview = &*view.inner().cast::<WKWebView>();
        webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        webview.setUIDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    }
    DELEGATES.with(|d| d.borrow_mut().insert(label, delegate));
}

pub fn cleanup(label: &str) {
    DELEGATES.with(|d| d.borrow_mut().remove(label));
}

pub fn screenshot(view: tauri::webview::PlatformWebview, sender: ImageSender) {
    let sender = Mutex::new(Some(sender));
    let callback = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
        let result = if !error.is_null() {
            Err(unsafe { &*error }.localizedDescription().to_string())
        } else if image.is_null() {
            Err(super::failure("WebKit returned no image"))
        } else {
            png(unsafe { &*image })
        };
        if let Ok(mut sender) = sender.lock()
            && let Some(sender) = sender.take()
        {
            let _ = sender.send(result);
        }
    });
    unsafe {
        (&*view.inner().cast::<WKWebView>())
            .takeSnapshotWithConfiguration_completionHandler(None, &callback);
    }
}

fn png(image: &NSImage) -> Result<Vec<u8>, String> {
    let data = image
        .TIFFRepresentation()
        .ok_or_else(|| super::failure("WebKit image could not be decoded"))?;
    let bitmap = NSBitmapImageRep::imageRepWithData(&data)
        .ok_or_else(|| super::failure("WebKit bitmap could not be decoded"))?;
    let png = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .ok_or_else(|| super::failure("WebKit PNG encoding failed"))?;
    Ok(png.to_vec())
}

// Wry queues eval calls until its navigation delegate reports a finished load.
// Our restrictive delegate owns that callback, so use WK's evaluation API
// directly rather than leaving readiness callbacks in Wry's pending queue.
pub fn evaluate(
    view: tauri::webview::PlatformWebview,
    js: &str,
    sender: tokio::sync::oneshot::Sender<Result<String, String>>,
) {
    let sender = Mutex::new(Some(sender));
    let callback = RcBlock::new(move |value: *mut AnyObject, error: *mut NSError| {
        let result = if !error.is_null() {
            Err(unsafe { &*error }.localizedDescription().to_string())
        } else {
            unsafe { value.as_ref() }
                .and_then(|value| value.downcast_ref::<NSString>())
                .map(|value| value.to_string())
                .ok_or_else(|| super::failure("Evaluation returned no JSON"))
        };
        if let Ok(mut sender) = sender.lock()
            && let Some(sender) = sender.take()
        {
            let _ = sender.send(result);
        }
    });
    unsafe {
        (&*view.inner().cast::<WKWebView>()).evaluateJavaScript_completionHandler(
            &NSString::from_str(&format!("JSON.stringify({js})")),
            Some(&callback),
        );
    }
}
