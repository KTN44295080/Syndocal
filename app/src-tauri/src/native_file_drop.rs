//! Resolve WebView2 DOM Files without enabling wry's Windows OLE drop
//! interceptor (which breaks HTML5 profile/scene/FX/Timeline dragging).
//! This adapter never imports media or accepts a path supplied in JSON.
use serde::Deserialize;
use tauri::Url;
use webview2_com::{
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2, ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs2,
    },
    WebMessageReceivedEventHandler,
};
use windows::core::{Interface, HSTRING, PWSTR};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DropRequest {
    #[serde(rename = "type")]
    kind: String,
    id: u32,
}

pub fn application_origin(config: &tauri::Config, development: bool) -> Url {
    // Match Tauri 2's AppManager::get_url for the main app. Reading the
    // WebView's current Source during setup can still observe about:blank.
    if development {
        if let Some(url) = &config.build.dev_url {
            return url.clone();
        }
    } else if let Some(tauri::utils::config::FrontendDist::Url(url)) = &config.build.frontend_dist {
        return url.clone();
    }
    let https = config
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .is_some_and(|window| window.use_https_scheme);
    Url::parse(if https {
        "https://tauri.localhost/"
    } else {
        "http://tauri.localhost/"
    })
    .expect("the embedded application origin is a valid URL")
}

fn trusted_source(source: &str, expected: &Url) -> bool {
    Url::parse(source).is_ok_and(|source| {
        source.scheme() == expected.scheme()
            && source.host_str() == expected.host_str()
            && source.port_or_known_default() == expected.port_or_known_default()
            && source.username().is_empty()
            && source.password().is_none()
    })
}

fn drop_request(json: &str) -> Option<DropRequest> {
    if json.len() > 128 {
        return None;
    }
    serde_json::from_str::<DropRequest>(json)
        .ok()
        .filter(|request| request.kind == "syndocal:file-drop" && request.id > 0)
}

pub fn install(core: &ICoreWebView2, expected: Url) -> windows::core::Result<()> {
    let handler = WebMessageReceivedEventHandler::create(Box::new(move |sender, args| unsafe {
        let (Some(sender), Some(args)) = (sender, args) else {
            return Ok(());
        };
        // Ordinary Tauri IPC has no AdditionalObjects. Do not copy/parse its
        // potentially large JSON on the UI thread just to recognize a drop.
        let Ok(object_args) = args.cast::<ICoreWebView2WebMessageReceivedEventArgs2>() else {
            return Ok(());
        };
        let objects = object_args.AdditionalObjects()?;
        let mut count = 0;
        objects.Count(&mut count)?;
        if count == 0 {
            return Ok(());
        }
        let mut json = PWSTR::null();
        args.WebMessageAsJson(&mut json)?;
        let Some(request) = drop_request(&webview2_com::take_pwstr(json)) else {
            // Includes ordinary Tauri IPC strings. This listener does not own them.
            return Ok(());
        };
        let mut source = PWSTR::null();
        args.Source(&mut source)?;
        if !trusted_source(&webview2_com::take_pwstr(source), &expected) {
            return Ok(());
        }
        let mut current_source = PWSTR::null();
        sender.Source(&mut current_source)?;
        if !trusted_source(&webview2_com::take_pwstr(current_source), &expected) {
            return Ok(());
        }
        let paths = (|| -> windows::core::Result<Vec<String>> {
            if count > 4096 {
                return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    0x80070057u32 as i32,
                )));
            }
            let mut paths = Vec::with_capacity(count as usize);
            for index in 0..count {
                let file: ICoreWebView2File = objects.GetValueAtIndex(index)?.cast()?;
                let mut path = PWSTR::null();
                file.Path(&mut path)?;
                let path = webview2_com::take_pwstr(path);
                if !std::path::Path::new(&path).is_absolute() {
                    return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                        0x80070057u32 as i32,
                    )));
                }
                paths.push(path);
            }
            Ok(paths)
        })();
        let reply = match paths {
            Ok(paths) => {
                serde_json::json!({ "type": "syndocal:file-drop-result", "id": request.id, "paths": paths })
            }
            Err(_) => {
                serde_json::json!({ "type": "syndocal:file-drop-result", "id": request.id, "error": true })
            }
        };
        sender.PostWebMessageAsJson(&HSTRING::from(reply.to_string()))?;
        Ok(())
    }));
    let mut token = 0;
    // The COM event source owns this handler until the WebView is destroyed;
    // the handler captures no WebView/window and creates no reference cycle.
    unsafe { core.add_WebMessageReceived(&handler, &mut token) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_origin_is_configured_before_the_first_navigation() {
        let mut config = tauri::Config::default();
        assert_eq!(
            application_origin(&config, false).as_str(),
            "http://tauri.localhost/"
        );
        assert!(!trusted_source(
            "about:blank",
            &application_origin(&config, false)
        ));
        config.build.dev_url = Some(Url::parse("http://localhost:5173/").unwrap());
        assert_eq!(
            application_origin(&config, true).as_str(),
            "http://localhost:5173/"
        );
        assert_eq!(
            application_origin(&config, false).as_str(),
            "http://tauri.localhost/"
        );
        let mut window = tauri::utils::config::WindowConfig::default();
        window.label = "main".into();
        window.use_https_scheme = true;
        config.app.windows.push(window);
        assert_eq!(
            application_origin(&config, false).as_str(),
            "https://tauri.localhost/"
        );
        config.build.frontend_dist = Some(tauri::utils::config::FrontendDist::Url(
            Url::parse("https://app.example/").unwrap(),
        ));
        assert_eq!(
            application_origin(&config, false).as_str(),
            "https://app.example/"
        );
    }

    #[test]
    fn request_rejects_paths_and_retired_or_malformed_messages() {
        assert!(drop_request(r#"{"type":"syndocal:file-drop","id":1}"#).is_some());
        for json in [
            r#"{"type":"syndocal:file-drop","id":1,"paths":["C:\\x.mp4"]}"#,
            r#"{"type":"syndocal:file-drop","id":0}"#,
            r#"{"type":"syndocal:file-drop","id":-1}"#,
            r#"{"type":"file-drop","id":1}"#,
            r#""tauri ipc string""#,
        ] {
            assert!(drop_request(json).is_none(), "{json}");
        }
    }

    #[test]
    fn source_is_bound_to_the_installed_application_origin() {
        let expected = Url::parse("http://tauri.localhost/").unwrap();
        assert!(trusted_source(
            "http://tauri.localhost/?pane=media",
            &expected
        ));
        for source in [
            "https://tauri.localhost/",
            "http://tauri.localhost.evil/",
            "http://tauri.localhost:5173/",
            "http://user@tauri.localhost/",
            "file:///C:/x.html",
            "broken",
        ] {
            assert!(!trusted_source(source, &expected), "{source}");
        }
        let dev = Url::parse("http://localhost:5173/").unwrap();
        assert!(trusted_source("http://localhost:5173/?x=1", &dev));
        assert!(!trusted_source("http://localhost:5174/", &dev));
    }
}
