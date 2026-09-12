use super::{ImageSender, NavigationError};
use std::{cell::RefCell, rc::Rc};
use webview2_com::{
    BasicAuthenticationRequestedEventHandler, CapturePreviewCompletedHandler,
    ClientCertificateRequestedEventHandler, Microsoft::Web::WebView2::Win32::*,
    NavigationCompletedEventHandler, PermissionRequestedEventHandler,
};
use windows::{
    Win32::{
        System::Com::{IStream, STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET},
        UI::Shell::SHCreateMemStream,
    },
    core::{BOOL, Interface},
};

pub const RENDERER: &str = "WebView2";

pub fn configure(view: tauri::webview::PlatformWebview, error: NavigationError, _label: String) {
    let result = (|| -> windows::core::Result<()> {
        // All COM access stays on Tauri's UI thread.
        unsafe {
            let webview = view.controller().CoreWebView2()?;
            let settings = webview.Settings()?;
            settings.SetIsWebMessageEnabled(false)?;
            settings.SetAreHostObjectsAllowed(false)?;
            settings.SetAreDefaultScriptDialogsEnabled(false)?;
            let mut token = 0;
            webview.add_PermissionRequested(
                &PermissionRequestedEventHandler::create(Box::new(|_, args| {
                    if let Some(args) = args {
                        args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;
            webview
                .cast::<ICoreWebView2_10>()?
                .add_BasicAuthenticationRequested(
                    &BasicAuthenticationRequestedEventHandler::create(Box::new(|_, args| {
                        if let Some(args) = args {
                            args.SetCancel(true)?;
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            webview
                .cast::<ICoreWebView2_5>()?
                .add_ClientCertificateRequested(
                    &ClientCertificateRequestedEventHandler::create(Box::new(|_, args| {
                        if let Some(args) = args {
                            args.SetCancel(true)?;
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            let errors = error.clone();
            webview.add_NavigationCompleted(
                &NavigationCompletedEventHandler::create(Box::new(move |_, args| {
                    if let Some(args) = args {
                        let mut success = BOOL::default();
                        args.IsSuccess(&mut success)?;
                        if !success.as_bool() {
                            let mut status = COREWEBVIEW2_WEB_ERROR_STATUS::default();
                            args.WebErrorStatus(&mut status)?;
                            if let Ok(mut error) = errors.lock() {
                                *error = Some(super::failure(format!(
                                    "Navigation failed: {}",
                                    status.0
                                )));
                            }
                        }
                    }
                    Ok(())
                })),
                &mut token,
            )?;
        }
        Ok(())
    })();
    if let Err(e) = result
        && let Ok(mut error) = error.lock()
    {
        *error = Some(e.to_string());
    }
}

pub fn cleanup(_label: &str) {}

pub fn screenshot(view: tauri::webview::PlatformWebview, sender: ImageSender) {
    let sender = Rc::new(RefCell::new(Some(sender)));
    let callback_sender = Rc::clone(&sender);
    let result = (|| -> Result<(), String> {
        let stream = unsafe { SHCreateMemStream(None) }
            .ok_or_else(|| super::failure("Could not allocate screenshot stream"))?;
        let captured = stream.clone();
        let callback = CapturePreviewCompletedHandler::create(Box::new(move |result| {
            let bytes = result
                .map_err(|e| e.to_string())
                .and_then(|_| read_stream(&captured));
            if let Some(sender) = callback_sender.borrow_mut().take() {
                let _ = sender.send(bytes);
            }
            Ok(())
        }));
        unsafe {
            view.controller()
                .CoreWebView2()
                .map_err(|e| e.to_string())?
                .CapturePreview(
                    COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
                    &stream,
                    &callback,
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(error) = result
        && let Some(sender) = sender.borrow_mut().take()
    {
        let _ = sender.send(Err(error));
    }
}

fn read_stream(stream: &IStream) -> Result<Vec<u8>, String> {
    unsafe {
        let mut stat = STATSTG::default();
        stream
            .Stat(&mut stat, STATFLAG_NONAME)
            .map_err(|e| e.to_string())?;
        let length = u32::try_from(stat.cbSize).map_err(|e| e.to_string())?;
        // A fixed viewport must never require unbounded allocation.
        if length > 64 * 1024 * 1024 {
            return Err(super::failure("Screenshot stream exceeds 64 MiB"));
        }
        let mut bytes = vec![0; length as usize];
        let mut read = 0;
        stream
            .Seek(0, STREAM_SEEK_SET, None)
            .map_err(|e| e.to_string())?;
        stream
            .Read(bytes.as_mut_ptr().cast(), length, Some(&mut read))
            .ok()
            .map_err(|e| e.to_string())?;
        bytes.truncate(read as usize);
        Ok(bytes)
    }
}
