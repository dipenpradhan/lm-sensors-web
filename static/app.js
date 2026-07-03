(() => {
  /* ── State ─────────────────────────────────────────────── */
  const grid   = document.getElementById('grid');
  const filter = document.getElementById('filter');
  const status = document.getElementById('status');

  let ws = null;
  let wsUrl = null;
  let reconnectMs = 1500;
  let allChips = new Map();   // name → ChipReadings

  /* ── Helpers ───────────────────────────────────────────── */
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

  function renderCard(chip) {
    const { chip: c, features } = chip;
    const id = c.name;
    let el = grid.querySelector(`[data-chip="${CSS.escape(id)}"]`);

    if (!el) {
      el = document.createElement('div');
      el.className = 'card expanded';
      el.setAttribute('data-chip', id);
      el.innerHTML = `
        <div class="card-header">
          <span class="card-title">${esc(c.name)}</span>
          <span class="card-meta">${esc(c.bus)}</span>
          <span class="card-chevron">▼</span>
        </div>
        <div class="card-body"></div>`;
      grid.appendChild(el);

      el.querySelector('.card-header').addEventListener('click', () => {
        el.classList.toggle('expanded');
      });
    }

    const body = el.querySelector('.card-body');
    const q = filter.value.toLowerCase();

    const rows = features.flatMap(f => {
      const fname = f.name.toLowerCase();
      if (q && !c.name.toLowerCase().includes(q) && !fname.includes(q)) return [];
      return f.sub_features.map(s => {
        if (q && !s.name.toLowerCase().includes(q) && !c.name.toLowerCase().includes(q)) return null;
        const uc = unitClass(s.unit);
        const val = s.value !== null && s.value !== undefined
          ? `${s.value} ${s.unit ?? ''}`
          : '—';
        return `<div class="feature"><span class="feature-name">${esc(s.name)}</span><span class="feature-val ${uc}">${esc(val)}</span></div>`;
      }).filter(Boolean);
    }).join('');

    body.innerHTML = rows || '<div class="empty">No readable features</div>';
  }

  function esc(s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  }

  function applyFilter() {
    const q = filter.value.toLowerCase();
    allChips.forEach(chip => renderCard(chip));
    grid.querySelectorAll('.card').forEach(el => {
      const q = filter.value.toLowerCase();
      if (!q) { el.style.display = ''; return; }
      const name = el.getAttribute('data-chip').toLowerCase();
      const visible = name.includes(q) ||
        [...el.querySelectorAll('.feature-name')].some(n => n.textContent.toLowerCase().includes(q));
      el.style.display = visible ? '' : 'none';
    });
  }

  /* ── Data feed ─────────────────────────────────────────── */
  function onReadings(data) {
    const chips = data.chips || [];
    chips.forEach(c => allChips.set(c.chip.name, c));
    allChips.forEach(chip => renderCard(chip));
    if (allChips.size === 0) grid.innerHTML = '<div class="empty">No sensors detected</div>';
  }

  /* ── WebSocket ─────────────────────────────────────────── */
  function connectWS() {
    if (!wsUrl) return;
    ws = new WebSocket(wsUrl);
    ws.binaryType = 'text';
    ws.onmessage = e => {
      try { onReadings(JSON.parse(e.data)); }
      catch (_) {}
    };
    ws.onclose = () => {
      status.className = 'status disconnected';
      status.querySelector('.label').textContent = 'Disconnected';
      setTimeout(connectWS, reconnectMs);
    };
    ws.onopen = () => {
      status.className = 'status connected';
      status.querySelector('.label').textContent = 'Live';
      reconnectMs = 1500;
    };
  }

  /* ── Fallback REST polling ────────────────────────────── */
  async function pollOnce() {
    try {
      const r = await fetch('/api/sensors');
      if (!r.ok) return;
      const data = await r.json();
      onReadings(data);
    } catch (_) {}
  }

  /* ── Init ──────────────────────────────────────────────── */
  function init() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    wsUrl = `${protocol}//${location.host}/ws/sensors`;

    filter.addEventListener('input', applyFilter);

    // Try WS first, fallback to polling
    connectWS();
    pollOnce();

    // Keep polling in background for safety
    setInterval(pollOnce, 10_000);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
})();
