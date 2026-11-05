//! Browser chrome (tab strip + address bar) rendered as a child WebView.
//! Communicates with Rust over wry IPC (`window.ipc.postMessage`).

pub const TOOLBAR_HEIGHT: u32 = 84; // tab strip 36 + nav bar 48

pub const TOOLBAR_HTML: &str = r##"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Segoe UI', system-ui, sans-serif; }
  body { background: #1b1d22; color: #e6e6e6; user-select: none; overflow: hidden; }
  #tabstrip { display: flex; align-items: center; height: 36px; background: #14151a; padding: 4px 6px 0 6px; gap: 4px; }
  .tab { display: flex; align-items: center; gap: 6px; height: 32px; padding: 0 10px; max-width: 200px; min-width: 120px;
         background: #23252d; border-radius: 8px 8px 0 0; font-size: 12px; cursor: default; }
  .tab.active { background: #2e313a; }
  .tab .title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tab .close { border: none; background: transparent; color: #aaa; cursor: pointer; font-size: 13px; border-radius: 4px; padding: 0 4px; }
  .tab .close:hover { background: #454855; color: #fff; }
  #newtab { border: none; background: transparent; color: #ccc; font-size: 18px; cursor: pointer; padding: 0 8px; }
  #nav { display: flex; align-items: center; gap: 6px; height: 48px; padding: 6px 8px; background: #2e313a; }
  button.navbtn { width: 32px; height: 32px; border: none; border-radius: 6px; background: transparent; color: #ddd; font-size: 16px; cursor: pointer; }
  button.navbtn:hover { background: #3d4049; }
  #omnibox { flex: 1; height: 32px; border: none; border-radius: 16px; background: #1b1d22; color: #eee;
             padding: 0 16px; font-size: 13px; outline: 1px solid #3d4049; }
  #omnibox:focus { outline: 2px solid #4f8cff; }
</style>
</head>
<body>
<div id="tabstrip"></div>
<div id="nav">
  <button class="navbtn" id="back" title="Back">&#8592;</button>
  <button class="navbtn" id="forward" title="Forward">&#8594;</button>
  <button class="navbtn" id="reload" title="Reload">&#10227;</button>
  <button class="navbtn" id="home" title="Home">&#8962;</button>
  <input id="omnibox" placeholder="Search Google or enter URL" autocomplete="off" spellcheck="false">
  <button class="navbtn" id="bookmark" title="Bookmark this page">&#9734;</button>
  <button class="navbtn" id="history" title="History">&#128340;</button>
  <button class="navbtn" id="bookmarks" title="Bookmarks">&#9733;</button>
  <button class="navbtn" id="downloads" title="Downloads">&#11015;</button>
  <button class="navbtn" id="private" title="New private tab">&#128377;</button>
  <button class="navbtn" id="settings" title="Settings">&#9881;</button>
  <button class="navbtn" id="devtools" title="DevTools">&lt;/&gt;</button>
</div>
<script>
  const send = (obj) => window.ipc.postMessage(JSON.stringify(obj));
  document.getElementById('back').onclick = () => send({type: 'back'});
  document.getElementById('forward').onclick = () => send({type: 'forward'});
  document.getElementById('reload').onclick = () => send({type: 'reload'});
  document.getElementById('home').onclick = () => send({type: 'home'});
  document.getElementById('devtools').onclick = () => send({type: 'devtools'});
  document.getElementById('bookmark').onclick = () => send({type: 'bookmark'});
  document.getElementById('history').onclick = () => send({type: 'open-panel', kind: 'history'});
  document.getElementById('bookmarks').onclick = () => send({type: 'open-panel', kind: 'bookmarks'});
  document.getElementById('downloads').onclick = () => send({type: 'open-panel', kind: 'downloads'});
  document.getElementById('private').onclick = () => send({type: 'new-private-tab'});
  document.getElementById('settings').onclick = () => send({type: 'open-panel', kind: 'settings'});
  const omni = document.getElementById('omnibox');
  omni.addEventListener('keydown', e => {
    if (e.key === 'Enter') { send({type: 'navigate', input: omni.value}); }
  });
  function renderTabs(state) {
    const strip = document.getElementById('tabstrip');
    strip.innerHTML = '';
    for (const t of state.tabs) {
      const el = document.createElement('div');
      el.className = 'tab' + (t.id === state.active ? ' active' : '');
      el.innerHTML = '<span class="title"></span><button class="close">&times;</button>';
      el.querySelector('.title').textContent = t.title || 'New Tab';
      el.onclick = () => send({type: 'switch-tab', id: t.id});
      el.querySelector('.close').onclick = (e) => { e.stopPropagation(); send({type: 'close-tab', id: t.id}); };
      strip.appendChild(el);
    }
    const add = document.createElement('button');
    add.id = 'newtab'; add.textContent = '+'; add.title = 'New Tab';
    add.onclick = () => send({type: 'new-tab'});
    strip.appendChild(add);
    if (document.activeElement !== omni) omni.value = state.omnibox || '';
  }
  window.__renderTabs = renderTabs;
</script>
</body>
</html>"##;
