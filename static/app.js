(() => {
  'use strict';

  /* ── DOM refs ──────────────────────────────────────────── */
  const grid   = document.getElementById('grid');
  const filter = document.getElementById('filter');
  const status = document.getElementById('status');

  let ws       = null;
  let wsUrl    = null;
  let wsTimer  = null;
  let reconnectMs = 1500;
  const devices = new Map();  // name → DeviceReadings

  /* ── Debounce helper ──────────────────────────────────── */
  function debounce(fn, ms) {
    let t;
    return (...args) => { clearTimeout(t); t = setTimeout(() => fn(...args), ms); };
  }

  /* ── Escape HTML (XSS-safe) ──────────────────────────── */
  function esc(s) {
    const d = document.createElement('div');
    d.textContent = String(s);
    return d.innerHTML;
  }

  /* ── Determine CSS class from unit string ─────────────── */
  function unitClass(unit) {
    if (!unit) return 'other';
    const u = unit.toLowerCase();
    if (u.includes('°c') || u.includes('°f') || u === 'k') return 'temp';
    if (u === 'v') return 'volt';
    if (u === 'a') return 'current';
    if (u === 'w') return 'power';
    if (u === 'j') return 'energy';
    if (u.includes('rpm')) return 'rpm';
    if (u === '%') return 'percent';
    return 'other';
  }

  /* ── Render/update a device card ──────────────────────── */
  function renderCard(dev) {
    const d = dev.device;
    const features = dev.features || [];
    const id = d.name;
    const q  = filter.value.toLowerCase();

    let el = grid.querySelector(`[data-device="${CSS.escape(id)}"]`);

    if (!el) {
      el = document.createElement('article');
      el.className = 'card expanded';
      el.setAttribute('data-device', id);
      el.innerHTML = `
        <button class="card-header" aria-expanded="true" aria-label="Toggle ${esc(d.name)}">
          <span class="card-title">${esc(d.name)}</span>
          <span class="card-meta">${esc(d.bus)}</span>
          <span class="card-chevron" aria-hidden="true">▼</span>
        </button>
        <div class="card-body"></div>`;

      const btn = el.querySelector('.card-header');
      btn.addEventListener('click', () => {
        const expanded = el.classList.toggle('expanded');
        btn.setAttribute('aria-expanded', String(expanded));
      });

      grid.appendChild(el);
    }

    // Update body with latest readings
    const body = el.querySelector('.card-body');
    const rows = features.flatMap(f => {
      const fname = f.name.toLowerCase();
      if (q && !d.name.toLowerCase().includes(q) && !fname.includes(q)) return [];
      return (f.sub_features || []).map(s => {
        if (q && !s.name.toLowerCase().includes(q) && !d.name.toLowerCase().includes(q)) return null;
        const uc = unitClass(s.unit);
        const val = s.value != null ? `${s.value} ${s.unit || ''}` : '\u2014';
        return `<div class="feature"><span class="feature-name">${esc(s.name)}</span><span class="feature-val ${uc}">${esc(val)}</span></div>`;
      }).filter(Boolean);
    }).join('');

    body.innerHTML = rows || '<div class="empty">No readable features</div>';
  }

  /* ── Apply filter to all cards ────────────────────────── */
  function applyFilter() {
    const q = filter.value.toLowerCase();

    // Remove stale devices
    for (const [name, dev] of devices) {
      renderCard(dev);
    }

    // Show/hide cards based on filter
    grid.querySelectorAll('.card').forEach(el => {
      if (!q) { el.style.display = ''; return; }
      const name = el.getAttribute('data-device').toLowerCase();
      const visible = name.includes(q) ||
        [...el.querySelectorAll('.feature-name')].some(n => n.textContent.toLowerCase().includes(q));
      el.style.display = visible ? '' : 'none';
    });
  }

  /* ── Process incoming readings ────────────────────────── */
  function onReadings(data) {
    const devList = data.devices || [];
    devList.forEach(d => devices.set(d.device.name, d));
    devices.forEach(d => renderCard(d));
    if (devices.size === 0) {
      grid.innerHTML = '<div class="empty">No sensors detected</div>';
    }
  }

  /* ── WebSocket connection with backoff ────────────────── */
  function connectWS() {
    if (!wsUrl) return;

    // Clean up previous connection
    if (ws) {
      ws.onmessage = ws.onerror = ws.onclose = null;
      try { ws.close(); } catch(_) {}
    }

    ws = new WebSocket(wsUrl);
    ws.binaryType = 'text';

    ws.onmessage = e => {
      try { onReadings(JSON.parse(e.data)); }
      catch (_) {}
    };

    ws.onerror = () => { /* handled by onclose */ };

    ws.onclose = () => {
      status.className = 'status disconnected';
      status.querySelector('.label').textContent = 'Disconnected';
      // Exponential backoff up to 30 s
      reconnectMs = Math.min(reconnectMs * 1.5, 30_000);
      wsTimer = setTimeout(connectWS, reconnectMs);
    };

    ws.onopen = () => {
      status.className = 'status connected';
      status.querySelector('.label').textContent = 'Live';
      reconnectMs = 1500;
    };
  }

  /* ── REST polling fallback ────────────────────────────── */
  async function pollOnce() {
    try {
      const r = await fetch('/api/sensors');
      if (!r.ok) return;
      onReadings(await r.json());
    } catch (_) { /* silently ignore — WS is primary */ }
  }

  /* ── Cleanup on unload ────────────────────────────────── */
  function cleanup() {
    if (wsTimer) clearTimeout(wsTimer);
    if (ws) {
      ws.onmessage = ws.onerror = ws.onclose = null;
      try { ws.close(); } catch(_) {}
    }
  }

  /* ── Initialise ───────────────────────────────────────── */
  function init() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    wsUrl = `${protocol}//${location.host}/ws/sensors`;

    filter.addEventListener('input', debounce(applyFilter, 150));
    window.addEventListener('beforeunload', cleanup);

    // Initial data: WS primary, REST fallback
    connectWS();
    pollOnce();

    // Background safety polling every 30 s
    setInterval(pollOnce, 30_000);
  }

  if (document.readyState !== 'loading') init();
  else document.addEventListener('DOMContentLoaded', init);
})();
