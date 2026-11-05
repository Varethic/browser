mod toolbar;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use varethic_storage::Store;
use varethic_url::{classify_omnibox_input_with_search, OmniboxTarget, DEFAULT_SEARCH_URL};
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::{Rect, WebView, WebViewBuilder};

const HOME_URL: &str = "https://www.google.com/";
const NEW_TAB_URL: &str = "https://www.google.com/";

#[derive(Debug)]
enum UserEvent {
    Navigate(String),
    NewTab,
    CloseTab(u64),
    SwitchTab(u64),
    Back,
    Forward,
    Reload,
    Home,
    Devtools,
    Bookmark,
    OpenPanel(PanelKind),
    ClosePanel,
    Goto(String),
    SetSearch(String),
    NewPrivateTab,
    SetProfile(String),
    NewProfile(String),
    TitleChanged(u64, String),
    PageLoaded(u64, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelKind {
    History,
    Bookmarks,
    Downloads,
    Settings,
    Profiles,
}

struct Tab {
    id: u64,
    title: String,
    url: String,
    incognito: bool,
    webview: WebView,
}

fn navigate_target(input: &str, search_template: &str) -> String {
    match classify_omnibox_input_with_search(input, search_template) {
        OmniboxTarget::Navigate(u) => u.to_url_string(),
        OmniboxTarget::Search(u) => u,
    }
}

fn valid_profile_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 32
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn current_search_template(store: &Arc<Mutex<Store>>) -> String {
    store
        .lock()
        .ok()
        .and_then(|s| s.get_setting("search_url").ok().flatten())
        .filter(|t| varethic_url::is_valid_search_template(t))
        .unwrap_or_else(|| DEFAULT_SEARCH_URL.to_string())
}

fn panel_bounds(window: &tao::window::Window) -> Rect {
    let scale = window.scale_factor();
    let size = window.inner_size();
    let w = 440.0_f64;
    Rect {
        position: LogicalPosition::new(
            (size.width as f64 / scale) - w,
            toolbar::TOOLBAR_HEIGHT as f64,
        )
        .into(),
        size: LogicalSize::new(
            w,
            (size.height as f64 / scale) - toolbar::TOOLBAR_HEIGHT as f64,
        )
        .into(),
    }
}

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn profiles_panel_html(profiles_body: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>
:root {{ color-scheme: dark; }}
* {{ font-family: 'Segoe UI', sans-serif; margin: 0; }}
body {{ background: #1b1d22; color: #e6e6e6; }}
header {{ display: flex; justify-content: space-between; align-items: center; padding: 10px 12px; background: #14151a; }}
h1 {{ font-size: 14px; }}
button {{ background: transparent; border: none; color: #ddd; font-size: 16px; cursor: pointer; }}
.settings {{ padding: 14px; display: flex; flex-direction: column; gap: 8px; }}
label {{ font-size: 12px; }}
input {{ height: 30px; background: #14151a; color: #eee; border: 1px solid #3d4049; border-radius: 6px; padding: 0 10px; font-size: 12px; }}
.save {{ background: #4f8cff; color: #fff; border-radius: 6px; height: 30px; font-size: 12px; }}
.row {{ padding: 8px 12px; border-bottom: 1px solid #2a2c33; cursor: pointer; }}
.row:hover {{ background: #23252d; }}
.t {{ font-size: 12px; }}
.u {{ font-size: 11px; color: #9aa0ae; }}
</style></head><body>
<header><h1>Profiles</h1><button onclick="window.ipc.postMessage(JSON.stringify({{type:'close-panel'}}))">&times;</button></header>
{profiles_body}
<script>
function setProfile(name) {{ window.ipc.postMessage(JSON.stringify({{type:'set-profile', name: name}})); }}
function newProfile() {{
  const v = document.getElementById('newprofile').value.trim();
  if (v) window.ipc.postMessage(JSON.stringify({{type:'new-profile', name: v}}));
}}
</script>
</body></html>"#,
        profiles_body = profiles_body
    )
}

fn settings_panel_html(settings_body: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>
:root {{ color-scheme: dark; }}
* {{ font-family: 'Segoe UI', sans-serif; margin: 0; }}
body {{ background: #1b1d22; color: #e6e6e6; }}
header {{ display: flex; justify-content: space-between; align-items: center; padding: 10px 12px; background: #14151a; }}
h1 {{ font-size: 14px; }}
button {{ background: transparent; border: none; color: #ddd; font-size: 16px; cursor: pointer; }}
.settings {{ padding: 14px; display: flex; flex-direction: column; gap: 8px; }}
label {{ font-size: 12px; }}
.hint {{ font-size: 11px; color: #9aa0ae; }}
input {{ height: 30px; background: #14151a; color: #eee; border: 1px solid #3d4049; border-radius: 6px; padding: 0 10px; font-size: 12px; }}
.save {{ background: #4f8cff; color: #fff; border-radius: 6px; height: 30px; font-size: 12px; }}
</style></head><body>
<header><h1>Settings</h1><button onclick="window.ipc.postMessage(JSON.stringify({{type:'close-panel'}}))">&times;</button></header>
{settings_body}
<script>
function saveSearch() {{
  const v = document.getElementById('search').value;
  window.ipc.postMessage(JSON.stringify({{type:'set-search', value: v}}));
  document.getElementById('status').textContent = 'Saved (applies to new searches).';
}}
</script>
</body></html>"#,
        settings_body = settings_body
    )
}

fn panel_html(kind: PanelKind, store: &Arc<Mutex<Store>>, registry: &Arc<Mutex<Store>>) -> String {
    let (heading, rows) = match kind {
        PanelKind::History => {
            let items = store
                .lock()
                .map(|s| s.recent_history(200))
                .unwrap_or_else(|_| Ok(vec![]))
                .unwrap_or_default();
            let rows: String = items
                .iter()
                .map(|e| {
                    let url_js = serde_json::to_string(&e.url).unwrap_or_else(|_| "\"\"".into());
                    format!(
                        "<div class='row' onclick='goto({url_js})'><div class='t'>{}</div><div class='u'>{}</div></div>",
                        esc_html(if e.title.is_empty() { &e.url } else { &e.title }),
                        esc_html(&e.url)
                    )
                })
                .collect();
            ("History", rows)
        }
        PanelKind::Bookmarks => {
            let items = store
                .lock()
                .map(|s| s.list_bookmarks())
                .unwrap_or_else(|_| Ok(vec![]))
                .unwrap_or_default();
            let rows: String = items
                .iter()
                .map(|b| {
                    let url_js = serde_json::to_string(&b.url).unwrap_or_else(|_| "\"\"".into());
                    format!(
                        "<div class='row' onclick='goto({url_js})'><div class='t'>{}</div><div class='u'>{}</div></div>",
                        esc_html(if b.title.is_empty() { &b.url } else { &b.title }),
                        esc_html(&b.url)
                    )
                })
                .collect();
            ("Bookmarks", rows)
        }
        PanelKind::Profiles => {
            let (profiles, current) = registry
                .lock()
                .map(|r| {
                    (
                        r.list_profiles().unwrap_or_default(),
                        r.get_setting("current_profile")
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| "Default".into()),
                    )
                })
                .unwrap_or_else(|_| (vec!["Default".into()], "Default".into()));
            let list_rows: String = profiles
                .iter()
                .map(|p| {
                    let name_js = serde_json::to_string(p).unwrap_or_else(|_| "\"\"".into());
                    let marker = if *p == current { " \u{25CF}" } else { "" };
                    format!(
                        "<div class='row' onclick='setProfile({name_js})'><div class='t'>{}{}</div><div class='u'>Click to switch</div></div>",
                        esc_html(p),
                        marker
                    )
                })
                .collect();
            let body = format!(
                "<div class='settings'><label for='newprofile'>Create profile</label>
                 <input id='newprofile' type='text' placeholder='e.g. Work' maxlength='32' spellcheck='false'>
                 <button class='save' onclick='newProfile()'>Create & switch</button>
                 </div>
                 <div class='rows'>{list_rows}</div>"
            );
            return profiles_panel_html(&body);
        }
        PanelKind::Settings => {
            let current = store
                .lock()
                .ok()
                .and_then(|s| s.get_setting("search_url").ok().flatten())
                .unwrap_or_else(|| DEFAULT_SEARCH_URL.to_string());
            let body = format!(
                "<div class='settings'><label for='search'>Default search URL template</label>
                 <div class='hint'>Must be http(s) and contain {{searchTerms}} exactly once. Google is the default.</div>
                 <input id='search' type='text' value='{}' spellcheck='false'>
                 <button class='save' onclick='saveSearch()'>Save</button>
                 <div id='status' class='hint'></div></div>",
                esc_html(&current)
            );
            return settings_panel_html(&body);
        }
        PanelKind::Downloads => {
            let items = store
                .lock()
                .map(|s| s.list_downloads(100))
                .unwrap_or_else(|_| Ok(vec![]))
                .unwrap_or_default();
            let rows: String = items
                .iter()
                .map(|d| {
                    format!(
                        "<div class='row'><div class='t'>{}{}</div><div class='u'>{}</div></div>",
                        if d.success { "" } else { "[failed] " },
                        esc_html(&d.path),
                        esc_html(&d.url)
                    )
                })
                .collect();
            ("Downloads", rows)
        }
    };
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>
:root {{ color-scheme: dark; }}
* {{ font-family: 'Segoe UI', sans-serif; margin: 0; }}
body {{ background: #1b1d22; color: #e6e6e6; }}
header {{ display: flex; justify-content: space-between; align-items: center; padding: 10px 12px; background: #14151a; position: sticky; top: 0; }}
h1 {{ font-size: 14px; }}
button {{ background: transparent; border: none; color: #ddd; font-size: 16px; cursor: pointer; }}
.row {{ padding: 8px 12px; border-bottom: 1px solid #2a2c33; cursor: pointer; }}
.row:hover {{ background: #23252d; }}
.t {{ font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
.u {{ font-size: 11px; color: #9aa0ae; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
.empty {{ padding: 16px; color: #9aa0ae; font-size: 12px; }}
</style></head><body>
<header><h1>{heading}</h1><button onclick="window.ipc.postMessage(JSON.stringify({{type:'close-panel'}}))">&times;</button></header>
{body}
<script>function goto(url) {{ window.ipc.postMessage(JSON.stringify({{type:'goto', url: url}})); }}</script>
</body></html>"#,
        heading = heading,
        body = if rows.is_empty() {
            "<div class='empty'>Nothing here yet.</div>".to_string()
        } else {
            rows
        },
    )
}

fn content_bounds(window: &tao::window::Window) -> Rect {
    let scale = window.scale_factor();
    let size = window.inner_size();
    Rect {
        position: LogicalPosition::new(0.0, toolbar::TOOLBAR_HEIGHT as f64).into(),
        size: LogicalSize::new(
            size.width as f64 / scale,
            (size.height as f64 / scale) - toolbar::TOOLBAR_HEIGHT as f64,
        )
        .into(),
    }
}

fn full_width_toolbar_bounds(window: &tao::window::Window) -> Rect {
    let scale = window.scale_factor();
    let size = window.inner_size();
    Rect {
        position: LogicalPosition::new(0.0, 0.0).into(),
        size: LogicalSize::new(size.width as f64 / scale, toolbar::TOOLBAR_HEIGHT as f64).into(),
    }
}

fn sync_toolbar(toolbar_view: &WebView, tabs: &[Tab], active: Option<u64>, omnibox: &str) {
    let state = serde_json::json!({
        "tabs": tabs.iter().map(|t| serde_json::json!({
            "id": t.id,
            "title": t.title,
            "url": t.url,
        })).collect::<Vec<_>>(),
        "active": active,
        "omnibox": omnibox,
    });
    let js = format!("window.__renderTabs({});", state.to_string());
    let _ = toolbar_view.evaluate_script(&js);
}

fn main() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Varethic")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .build(&event_loop)
        .expect("failed to create window");

    // Persistent browser-data store (history, bookmarks, downloads).
    let data_dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Varethic");
    let _ = std::fs::create_dir_all(&data_dir);
    let downloads_dir = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Downloads");
    let _ = std::fs::create_dir_all(&downloads_dir);
    // Profile registry (tracks named profiles + current selection).
    let registry = Arc::new(Mutex::new(
        Store::open(&data_dir.join("profiles.db")).expect("failed to open profiles.db"),
    ));
    if let Ok(r) = registry.lock() {
        let _ = r.create_profile("Default");
    }
    let current_profile = registry
        .lock()
        .ok()
        .and_then(|r| r.get_setting("current_profile").ok().flatten())
        .unwrap_or_else(|| "Default".to_string());
    let profile_db_path_init =
        |name: &str| -> PathBuf { data_dir.join("profiles").join(name).join("varethic.db") };
    let legacy_db = data_dir.join("varethic.db");
    let default_db = profile_db_path_init(&current_profile);
    if legacy_db.exists() && !default_db.exists() {
        if let Some(parent) = default_db.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(&legacy_db, &default_db);
    }
    let store = Arc::new(Mutex::new(
        Store::open(&default_db).expect("failed to open profile varethic.db"),
    ));

    let profile_base = data_dir.clone();
    let profile_db_path = move |name: &str| -> PathBuf {
        profile_base.join("profiles").join(name).join("varethic.db")
    };

    // Toolbar (browser chrome) child webview.
    let ipc_proxy = proxy.clone();
    let toolbar_view = WebViewBuilder::new()
        .with_html(toolbar::TOOLBAR_HTML)
        .with_bounds(full_width_toolbar_bounds(&window))
        .with_ipc_handler(move |req| {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(req.body()) {
                let ev = match msg.get("type").and_then(|t| t.as_str()) {
                    Some("navigate") => msg
                        .get("input")
                        .and_then(|i| i.as_str())
                        .map(|s| UserEvent::Navigate(s.to_string())),
                    Some("new-tab") => Some(UserEvent::NewTab),
                    Some("new-private-tab") => Some(UserEvent::NewPrivateTab),
                    Some("close-tab") => msg
                        .get("id")
                        .and_then(|i| i.as_u64())
                        .map(UserEvent::CloseTab),
                    Some("switch-tab") => msg
                        .get("id")
                        .and_then(|i| i.as_u64())
                        .map(UserEvent::SwitchTab),
                    Some("back") => Some(UserEvent::Back),
                    Some("forward") => Some(UserEvent::Forward),
                    Some("reload") => Some(UserEvent::Reload),
                    Some("home") => Some(UserEvent::Home),
                    Some("devtools") => Some(UserEvent::Devtools),
                    Some("bookmark") => Some(UserEvent::Bookmark),
                    Some("open-panel") => msg
                        .get("kind")
                        .and_then(|k| k.as_str())
                        .and_then(|k| match k {
                            "history" => Some(PanelKind::History),
                            "bookmarks" => Some(PanelKind::Bookmarks),
                            "downloads" => Some(PanelKind::Downloads),
                            "settings" => Some(PanelKind::Settings),
                            "profiles" => Some(PanelKind::Profiles),
                            _ => None,
                        })
                        .map(UserEvent::OpenPanel),
                    _ => None,
                };
                if let Some(ev) = ev {
                    let _ = ipc_proxy.send_event(ev);
                }
            }
        })
        .build_as_child(&window)
        .expect("failed to create toolbar webview");

    let mut tabs: Vec<Tab> = Vec::new();
    let mut panel: Option<WebView> = None;
    let mut next_tab_id: u64 = 1;

    // Helper to spawn a new tab webview.
    let spawn_tab = |window: &tao::window::Window,
                     tab_id: u64,
                     url: &str,
                     proxy: &tao::event_loop::EventLoopProxy<UserEvent>,
                     store: Arc<Mutex<Store>>,
                     downloads_dir: PathBuf,
                     incognito: bool|
     -> WebView {
        let title_proxy = proxy.clone();
        let load_proxy = proxy.clone();
        let dl_completed_store = store.clone();
        WebViewBuilder::new()
            .with_url(url)
            .with_incognito(incognito)
            .with_bounds(content_bounds(window))
            .with_document_title_changed_handler(move |title| {
                let _ = title_proxy.send_event(UserEvent::TitleChanged(tab_id, title));
            })
            .with_on_page_load_handler(move |_event, url| {
                let _ = load_proxy.send_event(UserEvent::PageLoaded(tab_id, url));
            })
            .with_download_started_handler(move |url, path| {
                // Save downloads into the user's Downloads folder.
                let filename = url
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty() && s.contains('.'))
                    .unwrap_or("download")
                    .to_string();
                *path = downloads_dir.join(&filename);
                true
            })
            .with_download_completed_handler(move |url, path, success| {
                if let (Ok(store), Some(p)) = (dl_completed_store.lock(), path) {
                    let _ = store.record_download(&url, &p.to_string_lossy(), success);
                }
            })
            .build_as_child(window)
            .expect("failed to create tab webview")
    };

    // Open the first tab.
    let first = spawn_tab(
        &window,
        next_tab_id,
        NEW_TAB_URL,
        &proxy,
        store.clone(),
        downloads_dir.clone(),
        false,
    );
    tabs.push(Tab {
        id: next_tab_id,
        title: "New Tab".into(),
        url: NEW_TAB_URL.into(),
        incognito: false,
        webview: first,
    });
    let mut active_tab: Option<u64> = Some(next_tab_id);
    next_tab_id += 1;
    sync_toolbar(&toolbar_view, &tabs, active_tab, NEW_TAB_URL);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_) => {
                    let _ = toolbar_view.set_bounds(full_width_toolbar_bounds(&window));
                    let b = content_bounds(&window);
                    for t in &tabs {
                        let _ = t.webview.set_bounds(b);
                    }
                    if let Some(p) = &panel {
                        let _ = p.set_bounds(panel_bounds(&window));
                    }
                }
                _ => {}
            },
            Event::UserEvent(ev) => match ev {
                UserEvent::NewTab => {
                    let id = next_tab_id;
                    next_tab_id += 1;
                    let view = spawn_tab(
                        &window,
                        id,
                        NEW_TAB_URL,
                        &proxy,
                        store.clone(),
                        downloads_dir.clone(),
                        false,
                    );
                    // Hide previous active tab.
                    if let Some(active) = active_tab {
                        if let Some(t) = tabs.iter().find(|t| t.id == active) {
                            let _ = t.webview.set_visible(false);
                        }
                    }
                    tabs.push(Tab {
                        id,
                        title: "New Tab".into(),
                        url: NEW_TAB_URL.into(),
                        incognito: false,
                        webview: view,
                    });
                    active_tab = Some(id);
                    if let Some(t) = tabs.last() {
                        let _ = t.webview.set_visible(true);
                    }
                    sync_toolbar(&toolbar_view, &tabs, active_tab, NEW_TAB_URL);
                }
                UserEvent::NewPrivateTab => {
                    let id = next_tab_id;
                    next_tab_id += 1;
                    let view = spawn_tab(
                        &window,
                        id,
                        NEW_TAB_URL,
                        &proxy,
                        store.clone(),
                        downloads_dir.clone(),
                        true,
                    );
                    if let Some(active) = active_tab {
                        if let Some(t) = tabs.iter().find(|t| t.id == active) {
                            let _ = t.webview.set_visible(false);
                        }
                    }
                    tabs.push(Tab {
                        id,
                        title: "\u{1F576} New Private Tab".into(),
                        url: NEW_TAB_URL.into(),
                        incognito: true,
                        webview: view,
                    });
                    active_tab = Some(id);
                    if let Some(t) = tabs.last() {
                        let _ = t.webview.set_visible(true);
                    }
                    sync_toolbar(&toolbar_view, &tabs, active_tab, NEW_TAB_URL);
                }
                UserEvent::CloseTab(id) => {
                    if let Some(pos) = tabs.iter().position(|t| t.id == id) {
                        let closing_active = active_tab == Some(id);
                        tabs.remove(pos); // dropping the WebView destroys it
                        if tabs.is_empty() {
                            // Always keep at least one tab.
                            let nid = next_tab_id;
                            next_tab_id += 1;
                            let view = spawn_tab(
                                &window,
                                nid,
                                NEW_TAB_URL,
                                &proxy,
                                store.clone(),
                                downloads_dir.clone(),
                                false,
                            );
                            tabs.push(Tab {
                                id: nid,
                                title: "New Tab".into(),
                                url: NEW_TAB_URL.into(),
                                incognito: false,
                                webview: view,
                            });
                            active_tab = Some(nid);
                        } else if closing_active {
                            let new_active = tabs[pos.min(tabs.len() - 1)].id;
                            active_tab = Some(new_active);
                            for t in &tabs {
                                let _ = t.webview.set_visible(t.id == new_active);
                            }
                        }
                        let url = tabs
                            .iter()
                            .find(|t| Some(t.id) == active_tab)
                            .map(|t| t.url.clone())
                            .unwrap_or_default();
                        sync_toolbar(&toolbar_view, &tabs, active_tab, &url);
                    }
                }
                UserEvent::SwitchTab(id) => {
                    active_tab = Some(id);
                    for t in &tabs {
                        let _ = t.webview.set_visible(t.id == id);
                    }
                    let url = tabs
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| t.url.clone())
                        .unwrap_or_default();
                    sync_toolbar(&toolbar_view, &tabs, active_tab, &url);
                }
                UserEvent::Navigate(input) => {
                    let template = current_search_template(&store);
                    let url = navigate_target(&input, &template);
                    if let Some(active) = active_tab {
                        if let Some(t) = tabs.iter_mut().find(|t| t.id == active) {
                            let _ = t.webview.load_url(&url);
                            t.url = url.clone();
                        }
                    }
                    sync_toolbar(&toolbar_view, &tabs, active_tab, &url);
                }
                UserEvent::Back => {
                    if let Some(t) = tabs.iter().find(|t| Some(t.id) == active_tab) {
                        let _ = t.webview.go_back();
                    }
                }
                UserEvent::Forward => {
                    if let Some(t) = tabs.iter().find(|t| Some(t.id) == active_tab) {
                        let _ = t.webview.go_forward();
                    }
                }
                UserEvent::Reload => {
                    if let Some(t) = tabs.iter().find(|t| Some(t.id) == active_tab) {
                        let _ = t.webview.reload();
                    }
                }
                UserEvent::Home => {
                    if let Some(t) = tabs.iter_mut().find(|t| Some(t.id) == active_tab) {
                        let _ = t.webview.load_url(HOME_URL);
                        t.url = HOME_URL.into();
                    }
                    sync_toolbar(&toolbar_view, &tabs, active_tab, HOME_URL);
                }
                UserEvent::Devtools => {
                    if let Some(t) = tabs.iter().find(|t| Some(t.id) == active_tab) {
                        t.webview.open_devtools();
                    }
                }
                UserEvent::Bookmark => {
                    if let Some(t) = tabs.iter().find(|t| Some(t.id) == active_tab) {
                        if let Ok(store) = store.lock() {
                            let _ = store.add_bookmark(&t.url, &t.title);
                        }
                    }
                }
                UserEvent::SetSearch(template) => {
                    if varethic_url::is_valid_search_template(&template) {
                        if let Ok(store) = store.lock() {
                            let _ = store.set_setting("search_url", &template);
                        }
                        panel = None; // close settings panel on save
                    }
                }
                UserEvent::SetProfile(name) | UserEvent::NewProfile(name) => {
                    if !valid_profile_name(&name) {
                        return; // ignore invalid names
                    }
                    if let Ok(r) = registry.lock() {
                        let _ = r.create_profile(&name);
                        let _ = r.set_setting("current_profile", &name);
                    }
                    let path = profile_db_path(&name);
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Ok(new_store) = Store::open(&path) {
                        if let Ok(mut guard) = store.lock() {
                            *guard = new_store;
                        }
                    }
                    panel = None; // close profiles panel on switch
                }
                UserEvent::OpenPanel(kind) => {
                    // Replace any open panel.
                    panel = None;
                    let html = panel_html(kind, &store, &registry);
                    let panel_proxy = proxy.clone();
                    let bounds = panel_bounds(&window);
                    panel = WebViewBuilder::new()
                        .with_html(html)
                        .with_bounds(bounds)
                        .with_ipc_handler(move |req| {
                            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(req.body()) {
                                let ev = match msg.get("type").and_then(|t| t.as_str()) {
                                    Some("close-panel") => Some(UserEvent::ClosePanel),
                                    Some("set-search") => msg
                                        .get("value")
                                        .and_then(|v| v.as_str())
                                        .map(|v| UserEvent::SetSearch(v.to_string())),
                                    Some("set-profile") => msg
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .map(|v| UserEvent::SetProfile(v.to_string())),
                                    Some("new-profile") => msg
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .map(|v| UserEvent::NewProfile(v.to_string())),
                                    Some("goto") => msg
                                        .get("url")
                                        .and_then(|u| u.as_str())
                                        .map(|u| UserEvent::Goto(u.to_string())),
                                    _ => None,
                                };
                                if let Some(ev) = ev {
                                    let _ = panel_proxy.send_event(ev);
                                }
                            }
                        })
                        .build_as_child(&window)
                        .ok();
                }
                UserEvent::ClosePanel => {
                    panel = None;
                }
                UserEvent::Goto(url) => {
                    panel = None;
                    if let Some(t) = tabs.iter_mut().find(|t| Some(t.id) == active_tab) {
                        let _ = t.webview.load_url(&url);
                        t.url = url.clone();
                    }
                    sync_toolbar(&toolbar_view, &tabs, active_tab, &url);
                }
                UserEvent::TitleChanged(id, title) => {
                    if let Some(t) = tabs.iter_mut().find(|t| t.id == id) {
                        t.title = if t.incognito {
                            format!("\u{1F576} {}", title)
                        } else {
                            title
                        };
                    }
                    let url = tabs
                        .iter()
                        .find(|t| Some(t.id) == active_tab)
                        .map(|t| t.url.clone())
                        .unwrap_or_default();
                    sync_toolbar(&toolbar_view, &tabs, active_tab, &url);
                }
                UserEvent::PageLoaded(id, url) => {
                    let (title, incognito) = if let Some(t) = tabs.iter_mut().find(|t| t.id == id) {
                        t.url = url.clone();
                        (t.title.clone(), t.incognito)
                    } else {
                        (String::new(), false)
                    };
                    if !incognito && !url.starts_with("about:") && !url.starts_with("data:") {
                        if let Ok(store) = store.lock() {
                            let _ = store.record_visit(&url, &title);
                        }
                    }
                    let url = tabs
                        .iter()
                        .find(|t| Some(t.id) == active_tab)
                        .map(|t| t.url.clone())
                        .unwrap_or_default();
                    sync_toolbar(&toolbar_view, &tabs, active_tab, &url);
                }
            },
            _ => {}
        }
    });
}
