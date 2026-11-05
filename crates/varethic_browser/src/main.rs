mod toolbar;

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use varethic_url::{classify_omnibox_input, OmniboxTarget};
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
    TitleChanged(u64, String),
    PageLoaded(u64, String),
}

struct Tab {
    id: u64,
    title: String,
    url: String,
    webview: WebView,
}

fn navigate_target(input: &str) -> String {
    match classify_omnibox_input(input) {
        OmniboxTarget::Navigate(u) => u.to_url_string(),
        OmniboxTarget::Search(u) => u,
    }
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
    let mut next_tab_id: u64 = 1;

    // Helper to spawn a new tab webview.
    let spawn_tab = |window: &tao::window::Window,
                     tab_id: u64,
                     url: &str,
                     proxy: &tao::event_loop::EventLoopProxy<UserEvent>|
     -> WebView {
        let title_proxy = proxy.clone();
        let load_proxy = proxy.clone();
        WebViewBuilder::new()
            .with_url(url)
            .with_bounds(content_bounds(window))
            .with_document_title_changed_handler(move |title| {
                let _ = title_proxy.send_event(UserEvent::TitleChanged(tab_id, title));
            })
            .with_on_page_load_handler(move |_event, url| {
                let _ = load_proxy.send_event(UserEvent::PageLoaded(tab_id, url));
            })
            .build_as_child(window)
            .expect("failed to create tab webview")
    };

    // Open the first tab.
    let first = spawn_tab(&window, next_tab_id, NEW_TAB_URL, &proxy);
    tabs.push(Tab {
        id: next_tab_id,
        title: "New Tab".into(),
        url: NEW_TAB_URL.into(),
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
                }
                _ => {}
            },
            Event::UserEvent(ev) => match ev {
                UserEvent::NewTab => {
                    let id = next_tab_id;
                    next_tab_id += 1;
                    let view = spawn_tab(&window, id, NEW_TAB_URL, &proxy);
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
                            let view = spawn_tab(&window, nid, NEW_TAB_URL, &proxy);
                            tabs.push(Tab {
                                id: nid,
                                title: "New Tab".into(),
                                url: NEW_TAB_URL.into(),
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
                    let url = navigate_target(&input);
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
                UserEvent::TitleChanged(id, title) => {
                    if let Some(t) = tabs.iter_mut().find(|t| t.id == id) {
                        t.title = title;
                    }
                    let url = tabs
                        .iter()
                        .find(|t| Some(t.id) == active_tab)
                        .map(|t| t.url.clone())
                        .unwrap_or_default();
                    sync_toolbar(&toolbar_view, &tabs, active_tab, &url);
                }
                UserEvent::PageLoaded(id, url) => {
                    if let Some(t) = tabs.iter_mut().find(|t| t.id == id) {
                        t.url = url;
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
