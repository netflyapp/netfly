mod adblock;
mod bookmarks;
mod browser;
mod config;
mod downloads;
mod history;
mod inject;
mod paths;
mod sessions;
mod userscripts;

use adblock::Adblock;
use bookmarks::BookmarkStore;
use browser::BrowserState;
use config::Config;
use downloads::DownloadManager;
use history::History;
use inject::CONTENT_INIT_SCRIPT;
use parking_lot::Mutex;
use sessions::Session;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, RunEvent, State, WebviewUrl, Window,
};
use tauri::webview::{DownloadEvent, PageLoadEvent};
use url::Url;
use userscripts::UserScriptStore;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContentInsets {
    pub top: f64,
    pub left: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Default for ContentInsets {
    fn default() -> Self {
        Self {
            top: 44.0,
            left: 240.0,
            right: 0.0,
            bottom: 0.0,
        }
    }
}

#[derive(Debug, Default)]
struct Layout {
    insets: ContentInsets,
    overlay_open: bool,
}

struct AppState {
    browser: Mutex<BrowserState>,
    history: History,
    bookmarks: Mutex<BookmarkStore>,
    config: Mutex<Config>,
    userscripts: Mutex<UserScriptStore>,
    downloads: Mutex<DownloadManager>,
    adblock: Adblock,
    layout: Mutex<Layout>,
    /// Merged chord set (frontend-computed) swallowed by content webviews.
    chords: Mutex<Vec<String>>,
}

fn blank_url() -> Url {
    Url::parse("about:blank").expect("about:blank is valid")
}

fn content_bounds(
    window: &Window,
    insets: ContentInsets,
) -> tauri::Result<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let scale = window.scale_factor()?;
    let size = window.inner_size()?;
    let width = size.width as f64 / scale;
    let height = size.height as f64 / scale;
    let content_w = (width - insets.left - insets.right).max(100.0);
    let content_h = (height - insets.top - insets.bottom).max(100.0);
    Ok((
        LogicalPosition::new(insets.left, insets.top),
        LogicalSize::new(content_w, content_h),
    ))
}

fn chords_sync_js(chords: &[String]) -> String {
    let json = serde_json::to_string(chords).unwrap_or_else(|_| "[]".into());
    format!("window.__netflySetChords && window.__netflySetChords({json});")
}

fn sync_chords_to_all(app: &AppHandle, state: &AppState) {
    let js = chords_sync_js(&state.chords.lock());
    let ids: Vec<String> = state.browser.lock().tabs.iter().map(|t| t.id.clone()).collect();
    for id in ids {
        if let Some(wv) = app.get_webview(&id) {
            let _ = wv.eval(&js);
        }
    }
}

fn main_window(app: &AppHandle) -> Result<Window, String> {
    app.get_window("main")
        .ok_or_else(|| "main window missing".to_string())
}

fn emit_snapshot(app: &AppHandle, state: &AppState) {
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap);
}

fn parse_url(url: &str) -> Result<Url, String> {
    Url::parse(url).map_err(|e| format!("invalid url: {e}"))
}

fn create_content_webview(app: &AppHandle, label: &str, url: &str) -> Result<(), String> {
    if app.get_webview(label).is_some() {
        return Ok(());
    }
    let window = main_window(app)?;
    let insets = app
        .try_state::<Arc<AppState>>()
        .map(|s| s.layout.lock().insets)
        .unwrap_or_default();
    let (pos, size) = content_bounds(&window, insets).map_err(|e| e.to_string())?;
    let start = parse_url(url).unwrap_or_else(|_| blank_url());

    let app_nav = app.clone();
    let app_load = app.clone();
    let app_dl = app.clone();
    let tab_label = label.to_string();
    let builder = tauri::webview::WebviewBuilder::new(label, WebviewUrl::External(start))
        .auto_resize()
        .initialization_script(CONTENT_INIT_SCRIPT)
        .on_navigation(move |nav_url| handle_netfly_navigation(&app_nav, nav_url))
        .on_download(move |_webview, event| {
            handle_download(&app_dl, event)
        })
        .on_page_load(move |webview, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            let page_url = payload.url().to_string();
            // update tab meta + history
            if let Some(state) = app_load.try_state::<Arc<AppState>>() {
                {
                    let mut b = state.browser.lock();
                    if let Some(tab) = b.tabs.iter_mut().find(|t| t.id == tab_label) {
                        tab.url = page_url.clone();
                        tab.loading = false;
                        if tab.title.is_empty() || tab.title.starts_with("http") {
                            tab.title = page_url.clone();
                        }
                    }
                }
                let _ = state.history.record(&page_url, "");
                emit_snapshot(&app_load, &state);

                // push current shortcut chord set into the page
                let _ = webview.eval(chords_sync_js(&state.chords.lock()));

                // cosmetic adblock
                if let Some(js) = state.adblock.cosmetic_inject_js() {
                    let _ = webview.eval(js);
                }

                // inject matching userscripts
                if let Some(js) = state.userscripts.lock().inject_js_for(&page_url) {
                    let _ = webview.eval(js);
                }

                // pull document.title
                let _ = webview.eval(
                    r#"(function(){
                      try {
                        var t = document.title || '';
                        var u = location.href || '';
                        var f = document.createElement('iframe');
                        f.style.display='none';
                        f.src = 'netfly://page-meta?title=' + encodeURIComponent(t) + '&url=' + encodeURIComponent(u);
                        document.documentElement.appendChild(f);
                        setTimeout(function(){ try{f.remove()}catch(e){} }, 0);
                      } catch(e) {}
                    })();"#,
                );
            }
        });

    window
        .add_child(builder, pos, size)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn handle_netfly_navigation(app: &AppHandle, nav_url: &Url) -> bool {
    // Adblock: block top-level navigations to known ad/tracker hosts
    if nav_url.scheme() != "netfly" {
        if let Some(state) = app.try_state::<Arc<AppState>>() {
            let url_s = nav_url.as_str();
            if state.adblock.is_blocked_url(url_s) {
                state.adblock.record_block();
                state.browser.lock().status =
                    format!("Blocked {}", nav_url.host_str().unwrap_or("tracker"));
                emit_snapshot(app, &state);
                return false;
            }
        }
        return true;
    }

    let host = nav_url.host_str().unwrap_or("");
    // netfly://escape  OR  netfly://open-tab?url=...
    // Url parser: scheme netfly, host = first path segment for netfly://escape
    let cmd = if !host.is_empty() {
        host.to_string()
    } else {
        nav_url.path().trim_start_matches('/').to_string()
    };

    match cmd.as_str() {
        "escape" => {
            let _ = app.emit("browser://escape", ());
        }
        "shortcut" => {
            let chord = nav_url
                .query_pairs()
                .find(|(k, _)| k == "chord")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            if !chord.is_empty() {
                let _ = app.emit("browser://action", chord);
            }
        }
        "open-tab" => {
            let url = nav_url
                .query_pairs()
                .find(|(k, _)| k == "url")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            if !url.is_empty() {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let _ = open_in_new_tab(app, &state, &url);
                }
            }
        }
        "page-meta" => {
            let title = nav_url
                .query_pairs()
                .find(|(k, _)| k == "title")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            let url = nav_url
                .query_pairs()
                .find(|(k, _)| k == "url")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                {
                    let mut b = state.browser.lock();
                    let tab = b.active_mut();
                    if !url.is_empty() {
                        tab.url = url.clone();
                    }
                    if !title.is_empty() {
                        tab.title = title.clone();
                    }
                    tab.loading = false;
                }
                if !url.is_empty() {
                    let _ = state.history.record(&url, &title);
                }
                emit_snapshot(app, &state);
            }
        }
        _ => {}
    }
    false // cancel navigation
}

fn ensure_active_webview(app: &AppHandle, state: &AppState) -> Result<String, String> {
    let (id, url) = {
        let b = state.browser.lock();
        (b.active_id(), b.active().url.clone())
    };
    create_content_webview(app, &id, &url)?;
    show_only(app, state, &id)?;
    Ok(id)
}

fn show_only(app: &AppHandle, state: &AppState, active_id: &str) -> Result<(), String> {
    let ids: Vec<String> = state.browser.lock().tabs.iter().map(|t| t.id.clone()).collect();
    let window = main_window(app)?;
    let (insets, overlay_open) = {
        let layout = state.layout.lock();
        (layout.insets, layout.overlay_open)
    };
    let (pos, size) = content_bounds(&window, insets).map_err(|e| e.to_string())?;

    for id in ids {
        if let Some(wv) = app.get_webview(&id) {
            if id == active_id && !overlay_open {
                let _ = wv.set_position(pos);
                let _ = wv.set_size(size);
                let _ = wv.show();
            } else {
                let _ = wv.hide();
            }
        }
    }
    Ok(())
}

fn navigate_tab(app: &AppHandle, label: &str, url: &str) -> Result<(), String> {
    let parsed = parse_url(url)?;
    if let Some(wv) = app.get_webview(label) {
        wv.navigate(parsed).map_err(|e| e.to_string())?;
    } else {
        create_content_webview(app, label, url)?;
    }
    Ok(())
}

fn eval_active(app: &AppHandle, state: &AppState, js: &str) -> Result<(), String> {
    let id = ensure_active_webview(app, state)?;
    let webview = app
        .get_webview(&id)
        .ok_or_else(|| "content webview missing".to_string())?;
    webview.eval(js).map_err(|e| e.to_string())
}

fn focus_shell(app: &AppHandle) -> Result<(), String> {
    if let Some(shell) = app.get_webview("main") {
        let _ = shell.set_focus();
    }
    if let Ok(window) = main_window(app) {
        let _ = window.set_focus();
    }
    Ok(())
}

fn focus_content(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let id = ensure_active_webview(app, state)?;
    let webview = app
        .get_webview(&id)
        .ok_or_else(|| "content webview missing".to_string())?;
    webview.set_focus().map_err(|e| e.to_string())
}

fn resolve_url(state: &AppState, input: &str) -> String {
    let cfg = state.config.lock();
    config::normalize_url(input, &cfg)
}

fn handle_download(app: &AppHandle, event: DownloadEvent<'_>) -> bool {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return true;
    };
    match event {
        DownloadEvent::Requested { url, destination } => {
            let cfg = state.config.lock().clone();
            let suggested = destination
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            let dest = downloads::destination_for(&cfg, url.as_str(), suggested.as_deref());
            *destination = dest.clone();
            let item = state.downloads.lock().on_requested(url.as_str(), &dest);
            let _ = state.downloads.lock().persist();
            state.browser.lock().status = format!("↓ {}", item.filename);
            emit_snapshot(app, &state);
            let _ = app.emit("browser://download", item);
            true
        }
        DownloadEvent::Finished { url, path, success } => {
            let path_ref = path.as_ref().map(|p| p.as_path());
            // On macOS path is often None — recover last requested path for this url
            let recovered = if path_ref.is_none() {
                state
                    .downloads
                    .lock()
                    .list()
                    .into_iter()
                    .find(|i| {
                        i.url == url.as_str()
                            && matches!(i.status, downloads::DownloadStatus::Requested)
                    })
                    .map(|i| PathBuf::from(i.path))
            } else {
                None
            };
            let effective = path_ref.or(recovered.as_deref());
            let item = state
                .downloads
                .lock()
                .on_finished(url.as_str(), effective, success);
            let _ = state.downloads.lock().persist();
            if let Some(item) = item {
                state.browser.lock().status = if success {
                    format!("✓ {}", item.filename)
                } else {
                    format!("✗ {}", item.filename)
                };
                emit_snapshot(app, &state);
                let _ = app.emit("browser://download", item);
            }
            true
        }
        _ => true,
    }
}

fn open_in_new_tab(app: &AppHandle, state: &AppState, url: &str) -> Result<browser::BrowserSnapshot, String> {
    let normalized = resolve_url(state, url);
    let id = {
        let mut b = state.browser.lock();
        let id = b.push_tab(&normalized, &normalized);
        b.active_mut().loading = true;
        b.status = format!("Loading {normalized}");
        id
    };
    create_content_webview(app, &id, &normalized)?;
    show_only(app, state, &id)?;
    focus_shell(app)?;
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap.clone());
    Ok(snap)
}

fn record_history(state: &AppState, url: &str, title: &str) {
    let _ = state.history.record(url, title);
}

// --- Commands ---

#[tauri::command]
fn get_snapshot(state: State<'_, Arc<AppState>>) -> browser::BrowserSnapshot {
    state.browser.lock().snapshot()
}

#[tauri::command]
fn set_content_insets(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    top: f64,
    left: f64,
    right: f64,
    bottom: f64,
) -> Result<(), String> {
    state.layout.lock().insets = ContentInsets {
        top,
        left,
        right,
        bottom,
    };
    let id = state.browser.lock().active_id();
    show_only(&app, &state, &id)
}

#[tauri::command]
fn set_overlay(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    open: bool,
) -> Result<(), String> {
    state.layout.lock().overlay_open = open;
    let id = state.browser.lock().active_id();
    show_only(&app, &state, &id)?;
    if open {
        focus_shell(&app)?;
    } else {
        let _ = focus_content(&app, &state);
    }
    Ok(())
}

#[tauri::command]
fn set_active_chords(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    chords: Vec<String>,
) -> Result<(), String> {
    *state.chords.lock() = chords;
    sync_chords_to_all(&app, &state);
    Ok(())
}

#[tauri::command]
fn config_set_binding(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    action: String,
    chords: Vec<String>,
) -> Result<Config, String> {
    let cfg = {
        let mut cfg = state.config.lock();
        cfg.set_binding(&action, chords);
        cfg.save()?;
        cfg.clone()
    };
    let _ = app.emit("browser://config", cfg.clone());
    Ok(cfg)
}

#[tauri::command]
fn config_reset_binding(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    action: String,
) -> Result<Config, String> {
    let cfg = {
        let mut cfg = state.config.lock();
        cfg.reset_binding(&action);
        cfg.save()?;
        cfg.clone()
    };
    let _ = app.emit("browser://config", cfg.clone());
    Ok(cfg)
}

#[tauri::command]
fn config_set_ui(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    sidebar_width: u32,
    sidebar_collapsed: bool,
) -> Result<Config, String> {
    let cfg = {
        let mut cfg = state.config.lock();
        cfg.set_ui(sidebar_width, sidebar_collapsed);
        cfg.save()?;
        cfg.clone()
    };
    let _ = app.emit("browser://config", cfg.clone());
    Ok(cfg)
}

#[tauri::command]
fn config_set_general(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    start_page: Option<String>,
    default_search: Option<String>,
    restore_session: Option<bool>,
) -> Result<Config, String> {
    let cfg = {
        let mut cfg = state.config.lock();
        if let Some(v) = start_page {
            cfg.start_page = v;
        }
        if let Some(v) = default_search {
            cfg.default_search = v;
        }
        if let Some(v) = restore_session {
            cfg.restore_session = v;
        }
        cfg.save()?;
        cfg.clone()
    };
    let _ = app.emit("browser://config", cfg.clone());
    Ok(cfg)
}

#[tauri::command]
fn focus_page(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    focus_content(&app, &state)
}

#[tauri::command]
fn open_url(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: String,
    new_tab: Option<bool>,
) -> Result<browser::BrowserSnapshot, String> {
    if new_tab.unwrap_or(false) {
        return open_in_new_tab(&app, &state, &input);
    }
    let url = resolve_url(&state, &input);
    let id = {
        let mut b = state.browser.lock();
        let tab = b.active_mut();
        tab.url = url.clone();
        tab.loading = true;
        tab.title = url.clone();
        b.status = format!("Loading {url}");
        b.active_id()
    };
    create_content_webview(&app, &id, "about:blank")?;
    navigate_tab(&app, &id, &url)?;
    show_only(&app, &state, &id)?;
    focus_shell(&app)?;
    record_history(&state, &url, "");
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn tab_new(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<browser::BrowserSnapshot, String> {
    let start = state.config.lock().start_page.clone();
    open_in_new_tab(&app, &state, &start)
}

#[tauri::command]
fn tab_close(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<browser::BrowserSnapshot, String> {
    let (closed_id, active_id, closed_url) = {
        let mut b = state.browser.lock();
        let closed_url = b.active().url.clone();
        let (closed_id, active_id) = b.close_active();
        b.status = format!("Closed {closed_url}");
        (closed_id, active_id, closed_url)
    };
    if let Some(wv) = app.get_webview(&closed_id) {
        let _ = wv.close();
    }
    // ensure replacement tab exists as webview
    let url = state.browser.lock().active().url.clone();
    create_content_webview(&app, &active_id, &url)?;
    show_only(&app, &state, &active_id)?;
    focus_shell(&app)?;
    let _ = closed_url;
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn tab_undo_close(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<browser::BrowserSnapshot, String> {
    let restored = {
        let mut b = state.browser.lock();
        b.undo_close()
    };
    if let Some(tab) = restored {
        create_content_webview(&app, &tab.id, &tab.url)?;
        if tab.url != "about:blank" {
            navigate_tab(&app, &tab.id, &tab.url)?;
        }
        show_only(&app, &state, &tab.id)?;
        state.browser.lock().status = format!("Restored {}", tab.url);
    } else {
        state.browser.lock().status = "Nothing to undo".into();
    }
    focus_shell(&app)?;
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn tab_next(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<browser::BrowserSnapshot, String> {
    let id = state.browser.lock().next_tab();
    show_only(&app, &state, &id)?;
    focus_shell(&app)?;
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn tab_prev(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<browser::BrowserSnapshot, String> {
    let id = state.browser.lock().prev_tab();
    show_only(&app, &state, &id)?;
    focus_shell(&app)?;
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn tab_select(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    index: usize,
) -> Result<browser::BrowserSnapshot, String> {
    let id = state
        .browser
        .lock()
        .switch_tab(index)
        .ok_or_else(|| "invalid tab index".to_string())?;
    show_only(&app, &state, &id)?;
    focus_shell(&app)?;
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn go_back(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    eval_active(&app, &state, "window.history.back();")
}

#[tauri::command]
fn go_forward(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    eval_active(&app, &state, "window.history.forward();")
}

#[tauri::command]
fn reload(app: AppHandle, state: State<'_, Arc<AppState>>, hard: bool) -> Result<(), String> {
    if hard {
        eval_active(&app, &state, "location.reload();")
    } else {
        let id = ensure_active_webview(&app, &state)?;
        if let Some(wv) = app.get_webview(&id) {
            wv.reload().map_err(|e| e.to_string())
        } else {
            Err("content webview missing".into())
        }
    }
}

#[tauri::command]
fn scroll_by(app: AppHandle, state: State<'_, Arc<AppState>>, x: i32, y: i32) -> Result<(), String> {
    eval_active(&app, &state, &format!("window.scrollBy({x}, {y});"))
}

#[tauri::command]
fn scroll_to(app: AppHandle, state: State<'_, Arc<AppState>>, y: i32) -> Result<(), String> {
    eval_active(
        &app,
        &state,
        &format!("window.scrollTo({{ top: {y}, left: 0 }});"),
    )
}

#[tauri::command]
fn scroll_bottom(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    eval_active(
        &app,
        &state,
        "window.scrollTo({ top: document.body.scrollHeight, left: 0 });",
    )
}

#[tauri::command]
fn find_in_page(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    query: String,
    forward: bool,
) -> Result<(), String> {
    let q = serde_json::to_string(&query).map_err(|e| e.to_string())?;
    eval_active(
        &app,
        &state,
        &format!(
            "window.find({q}, false, {}, true, false, false, false);",
            !forward
        ),
    )
}

#[tauri::command]
fn yank_url(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let url = state.browser.lock().active().url.clone();
    let js = format!(
        "navigator.clipboard.writeText({});",
        serde_json::to_string(&url).unwrap_or_else(|_| "''".into())
    );
    if let Some(shell) = app.get_webview("main") {
        let _ = shell.eval(&js);
    }
    state.browser.lock().status = format!("Yanked {url}");
    emit_snapshot(&app, &state);
    Ok(url)
}

#[tauri::command]
fn set_status(app: AppHandle, state: State<'_, Arc<AppState>>, status: String) -> Result<(), String> {
    state.browser.lock().status = status;
    emit_snapshot(&app, &state);
    Ok(())
}

#[tauri::command]
fn report_page_meta(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    url: String,
    title: String,
) -> Result<(), String> {
    {
        let mut b = state.browser.lock();
        let tab = b.active_mut();
        if !url.is_empty() {
            tab.url = url.clone();
        }
        if !title.is_empty() {
            tab.title = title.clone();
        }
        tab.loading = false;
        b.status.clear();
    }
    if !url.is_empty() {
        record_history(&state, &url, &title);
    }
    emit_snapshot(&app, &state);
    Ok(())
}

#[tauri::command]
fn resize_content(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let id = ensure_active_webview(&app, &state)?;
    show_only(&app, &state, &id)
}

#[tauri::command]
fn history_search(
    state: State<'_, Arc<AppState>>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<history::HistoryEntry>, String> {
    state.history.search(&query, limit.unwrap_or(30))
}

#[tauri::command]
fn bookmark_set(
    state: State<'_, Arc<AppState>>,
    name: String,
    url: Option<String>,
) -> Result<(), String> {
    let target = url.unwrap_or_else(|| state.browser.lock().active().url.clone());
    state.bookmarks.lock().set_bookmark(&name, &target)?;
    state.browser.lock().status = format!("Bookmarked {name}");
    Ok(())
}

#[tauri::command]
fn bookmark_open(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    name: String,
    new_tab: Option<bool>,
) -> Result<browser::BrowserSnapshot, String> {
    let url = state
        .bookmarks
        .lock()
        .get_bookmark(&name)
        .ok_or_else(|| format!("no bookmark '{name}'"))?;
    open_url(app, state, url, new_tab)
}

#[tauri::command]
fn quickmark_set(state: State<'_, Arc<AppState>>, key: String) -> Result<(), String> {
    let url = state.browser.lock().active().url.clone();
    state.bookmarks.lock().set_quickmark(&key, &url)?;
    state.browser.lock().status = format!("Quickmark {key} → {url}");
    Ok(())
}

#[tauri::command]
fn quickmark_open(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    key: String,
) -> Result<browser::BrowserSnapshot, String> {
    let url = state
        .bookmarks
        .lock()
        .get_quickmark(&key)
        .ok_or_else(|| format!("no quickmark '{key}'"))?;
    open_url(app, state, url, Some(false))
}

#[tauri::command]
fn list_bookmarks(state: State<'_, Arc<AppState>>) -> Result<BookmarkStore, String> {
    Ok(state.bookmarks.lock().clone())
}

#[tauri::command]
fn session_save(state: State<'_, Arc<AppState>>, name: String) -> Result<(), String> {
    let (active, urls) = {
        let b = state.browser.lock();
        let urls: Vec<String> = b.tabs.iter().map(|t| t.url.clone()).collect();
        (b.active_tab, urls)
    };
    Session::save(&name, active, urls)?;
    state.browser.lock().status = format!("Session saved: {name}");
    Ok(())
}

#[tauri::command]
fn session_load(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    name: String,
) -> Result<browser::BrowserSnapshot, String> {
    let session = Session::load(&name)?;
    let old_ids: Vec<String> = {
        let b = state.browser.lock();
        b.tabs.iter().map(|t| t.id.clone()).collect()
    };
    for id in old_ids {
        if let Some(wv) = app.get_webview(&id) {
            let _ = wv.close();
        }
    }

    let mut fresh = BrowserState::empty();
    for url in &session.urls {
        let id = fresh.push_tab(url, url);
        create_content_webview(&app, &id, url)?;
    }
    if fresh.tabs.is_empty() {
        let id = fresh.push_tab("about:blank", "New Tab");
        create_content_webview(&app, &id, "about:blank")?;
    }
    let active = session.active.min(fresh.tabs.len().saturating_sub(1));
    fresh.active_tab = active;
    fresh.status = format!("Loaded session {name}");
    let active_id = fresh.active_id();
    *state.browser.lock() = fresh;

    show_only(&app, &state, &active_id)?;
    focus_shell(&app)?;
    let snap = state.browser.lock().snapshot();
    let _ = app.emit("browser://snapshot", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn data_path() -> Result<String, String> {
    paths::data_dir().map(|p| p.display().to_string())
}

#[tauri::command]
fn get_config(state: State<'_, Arc<AppState>>) -> Config {
    state.config.lock().clone()
}

#[tauri::command]
fn config_reload(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<Config, String> {
    state.config.lock().reload()?;
    let cfg = state.config.lock().clone();
    state.adblock.set_enabled(cfg.adblock);
    let _ = state.adblock.reload();
    state.browser.lock().status = "Config reloaded".into();
    emit_snapshot(&app, &state);
    let _ = app.emit("browser://config", cfg.clone());
    Ok(cfg)
}

#[tauri::command]
fn config_path() -> Result<String, String> {
    Config::path()
}

#[tauri::command]
fn config_edit(app: AppHandle) -> Result<String, String> {
    let path = paths::config_file()?;
    // Ensure file exists
    let _ = Config::load();
    // Open with default macOS editor/app for .toml
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn userscripts_list(state: State<'_, Arc<AppState>>) -> Result<Vec<userscripts::UserScript>, String> {
    Ok(state.userscripts.lock().scripts.clone())
}

#[tauri::command]
fn userscripts_reload(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    state.userscripts.lock().reload()?;
    let n = state.userscripts.lock().scripts.len();
    state.browser.lock().status = format!("Loaded {n} userscripts");
    emit_snapshot(&app, &state);
    Ok(n)
}

#[tauri::command]
fn userscripts_path() -> Result<String, String> {
    paths::userscripts_dir().map(|p| p.display().to_string())
}

#[tauri::command]
fn userscripts_open_dir(app: AppHandle) -> Result<String, String> {
    let dir = paths::userscripts_dir()?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(dir.display().to_string())
}

#[tauri::command]
fn downloads_list(state: State<'_, Arc<AppState>>) -> Vec<downloads::DownloadItem> {
    state.downloads.lock().list()
}

#[tauri::command]
fn downloads_clear(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.downloads.lock().clear_finished();
    let _ = state.downloads.lock().persist();
    state.browser.lock().status = "Downloads cleared".into();
    emit_snapshot(&app, &state);
    Ok(())
}

#[tauri::command]
fn downloads_open_dir(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let dir = state.config.lock().expanded_download_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(&dir, None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
fn downloads_open_file(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: u64,
) -> Result<String, String> {
    let item = state
        .downloads
        .lock()
        .list()
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| "download not found".to_string())?;
    if item.path.is_empty() {
        return Err("no path for download".into());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(&item.path, None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(item.path)
}

#[tauri::command]
fn download_url(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: String,
) -> Result<downloads::DownloadItem, String> {
    let url = resolve_url(&state, &input);
    let cfg = state.config.lock().clone();
    let dest = downloads::destination_for(&cfg, &url, None);
    let item = state.downloads.lock().on_requested(&url, &dest);
    emit_snapshot(&app, &state);
    let _ = app.emit("browser://download", item.clone());

    // run download off the main thread
    let app2 = app.clone();
    let url2 = url.clone();
    let dest2 = dest.clone();
    let item_id = item.id;
    std::thread::spawn(move || {
        let result = downloads::fetch_to_file(&url2, &dest2);
        if let Some(state) = app2.try_state::<Arc<AppState>>() {
            let success = result.is_ok();
            let status_msg = match &result {
                Ok(bytes) => format!("✓ download {bytes} bytes"),
                Err(e) => format!("✗ download failed: {e}"),
            };
            let item = state
                .downloads
                .lock()
                .on_finished(&url2, Some(dest2.as_path()), success);
            let _ = state.downloads.lock().persist();
            state.browser.lock().status = status_msg;
            emit_snapshot(&app2, &state);
            if let Some(item) = item {
                let _ = app2.emit("browser://download", item);
            }
            let _ = item_id;
        }
    });

    state.browser.lock().status = format!("↓ {}", item.filename);
    Ok(item)
}

#[tauri::command]
fn adblock_status(state: State<'_, Arc<AppState>>) -> adblock::AdblockStatus {
    state.adblock.status()
}

#[tauri::command]
fn adblock_set(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<adblock::AdblockStatus, String> {
    state.adblock.set_enabled(enabled);
    {
        let mut cfg = state.config.lock();
        cfg.adblock = enabled;
        let _ = cfg.save();
    }
    state.browser.lock().status = if enabled {
        "Adblock on".into()
    } else {
        "Adblock off".into()
    };
    emit_snapshot(&app, &state);
    Ok(state.adblock.status())
}

#[tauri::command]
fn adblock_reload(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    let n = state.adblock.reload()?;
    state.browser.lock().status = format!("Adblock hosts: {n}");
    emit_snapshot(&app, &state);
    Ok(n)
}

#[tauri::command]
fn adblock_open_list(app: AppHandle) -> Result<String, String> {
    let path = paths::blocklist_file()?;
    if !path.exists() {
        let _ = Adblock::load(true); // seeds file
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn quit_app(app: AppHandle, state: State<'_, Arc<AppState>>) {
    // auto-save last session
    let (active, urls) = {
        let b = state.browser.lock();
        let urls: Vec<String> = b.tabs.iter().map(|t| t.url.clone()).collect();
        (b.active_tab, urls)
    };
    let _ = Session::save_last(active, urls);
    app.exit(0);
}

fn bootstrap_first_tab(app: &AppHandle, state: &AppState) {
    let id = state.browser.lock().active_id();
    let _ = create_content_webview(app, &id, "about:blank");
    let _ = show_only(app, state, &id);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let history = History::open().expect("open history db");
    let bookmarks = BookmarkStore::load().unwrap_or_default();
    let cfg = Config::load().unwrap_or_default();
    let scripts = UserScriptStore::load().unwrap_or_default();
    let adblock = Adblock::load(cfg.adblock);
    let downloads = DownloadManager::load();
    let initial_insets = ContentInsets {
        left: if cfg.ui.sidebar_collapsed {
            0.0
        } else {
            cfg.ui.sidebar_width as f64
        },
        ..ContentInsets::default()
    };
    let state = Arc::new(AppState {
        browser: Mutex::new(BrowserState::default()),
        history,
        bookmarks: Mutex::new(bookmarks),
        config: Mutex::new(cfg),
        userscripts: Mutex::new(scripts),
        downloads: Mutex::new(downloads),
        adblock,
        layout: Mutex::new(Layout {
            insets: initial_insets,
            overlay_open: false,
        }),
        chords: Mutex::new(Vec::new()),
    });

    let state_for_exit = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            open_url,
            tab_new,
            tab_close,
            tab_undo_close,
            tab_next,
            tab_prev,
            tab_select,
            go_back,
            go_forward,
            reload,
            scroll_by,
            scroll_to,
            scroll_bottom,
            find_in_page,
            yank_url,
            set_status,
            report_page_meta,
            resize_content,
            set_content_insets,
            set_overlay,
            set_active_chords,
            config_set_binding,
            config_reset_binding,
            config_set_ui,
            config_set_general,
            focus_page,
            history_search,
            bookmark_set,
            bookmark_open,
            quickmark_set,
            quickmark_open,
            list_bookmarks,
            session_save,
            session_load,
            data_path,
            get_config,
            config_reload,
            config_path,
            config_edit,
            userscripts_list,
            userscripts_reload,
            userscripts_path,
            userscripts_open_dir,
            downloads_list,
            downloads_clear,
            downloads_open_dir,
            downloads_open_file,
            download_url,
            adblock_status,
            adblock_set,
            adblock_reload,
            adblock_open_list,
            quit_app,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<Arc<AppState>>().inner().clone();

            if let Some(win) = app.get_webview_window("main") {
                let h = handle.clone();
                let st = state.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::Resized(_) = event {
                        let id = st.browser.lock().active_id();
                        let _ = show_only(&h, &st, &id);
                    }
                });
            }

            let h2 = handle.clone();
            let st2 = state.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let restore = st2.config.lock().restore_session;
                // Restore last session if present, otherwise start page
                if restore {
                    if let Ok(Some(session)) = Session::load_last() {
                        if !session.urls.is_empty()
                            && !(session.urls.len() == 1 && session.urls[0] == "about:blank")
                        {
                            let mut fresh = BrowserState::empty();
                            for url in &session.urls {
                                let id = fresh.push_tab(url, url);
                                let _ = create_content_webview(&h2, &id, url);
                            }
                            let active = session.active.min(fresh.tabs.len().saturating_sub(1));
                            fresh.active_tab = active;
                            fresh.status = "Restored last session".into();
                            let active_id = fresh.active_id();
                            *st2.browser.lock() = fresh;
                            let _ = show_only(&h2, &st2, &active_id);
                            let _ = h2.emit("browser://snapshot", st2.browser.lock().snapshot());
                            return;
                        }
                    }
                }
                // start_page from config
                let start = st2.config.lock().start_page.clone();
                {
                    let mut b = st2.browser.lock();
                    if let Some(tab) = b.tabs.first_mut() {
                        tab.url = start.clone();
                        tab.title = if start == "about:blank" {
                            "New Tab".into()
                        } else {
                            start.clone()
                        };
                    }
                }
                bootstrap_first_tab(&h2, &st2);
                if start != "about:blank" {
                    if let Ok(id) = ensure_active_webview(&h2, &st2) {
                        let _ = navigate_tab(&h2, &id, &start);
                    }
                }
                let _ = h2.emit("browser://snapshot", st2.browser.lock().snapshot());
                let _ = h2.emit("browser://config", st2.config.lock().clone());
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Netfly")
        .run(move |_app_handle, event| {
            if let RunEvent::Exit = event {
                let (active, urls) = {
                    let b = state_for_exit.browser.lock();
                    let urls: Vec<String> = b.tabs.iter().map(|t| t.url.clone()).collect();
                    (b.active_tab, urls)
                };
                let _ = Session::save_last(active, urls);
            }
        });
}
