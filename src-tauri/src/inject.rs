//! JS injected into content webviews (escape bridge + shortcut forwarding).

/// Runs on every page; provides IPC via netfly:// pseudo-navigation.
///
/// The shell pushes the set of bound chords via `window.__netflySetChords`
/// (evaled by Rust). The capture-phase keydown listener swallows matching
/// chords and forwards them to the shell so app shortcuts work even while
/// the page has keyboard focus.
pub const CONTENT_INIT_SCRIPT: &str = r#"
(function () {
  if (window.__netflyInstalled) return;
  window.__netflyInstalled = true;

  var chords = new Set();
  window.__netflySetChords = function (list) {
    try { chords = new Set(list || []); } catch (_) {}
  };

  function ipc(path) {
    try {
      var f = document.createElement('iframe');
      f.style.display = 'none';
      f.src = 'netfly://' + path;
      document.documentElement.appendChild(f);
      setTimeout(function () {
        try { f.remove(); } catch (_) {}
      }, 0);
    } catch (_) {}
  }

  var KEY_MAP = {
    'arrowleft': 'left', 'arrowright': 'right',
    'arrowup': 'up', 'arrowdown': 'down',
    ' ': 'space', 'escape': 'esc'
  };

  function normalizeChord(e) {
    var key = (e.key || '').toLowerCase();
    if (key === 'meta' || key === 'control' || key === 'alt' || key === 'shift') return null;
    key = KEY_MAP[key] || key;
    var parts = [];
    if (e.metaKey) parts.push('cmd');
    if (e.ctrlKey) parts.push('ctrl');
    if (e.altKey) parts.push('alt');
    if (e.shiftKey) parts.push('shift');
    parts.push(key);
    return parts.join('+');
  }

  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') {
      ipc('escape');
      return;
    }
    var chord = normalizeChord(e);
    if (chord && chords.has(chord)) {
      e.preventDefault();
      e.stopPropagation();
      ipc('shortcut?chord=' + encodeURIComponent(chord));
    }
  }, true);
})();
"#;
