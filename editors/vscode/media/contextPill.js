(function () {
  const vscode = acquireVsCodeApi();
  const root = document.getElementById('pill');
  const loctreeChannel = root ? root.getAttribute('data-channel') : '';

  /** HTML-escape any value before it reaches innerHTML — the webview's only
   *  defence against a file path or finding message injecting markup. */
  function esc(s) {
    return String(s ?? '').replace(/[&<>"']/g, (c) => (
      { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]
    ));
  }
  /** Render a list of strings or {kind,name} symbols as escaped tag chips. */
  function tags(items, cls) {
    return (items || []).map((t) => `<span class="tag ${cls}">${esc(typeof t === 'string' ? t : (t.kind ? t.kind + ' ' + t.name : t.name))}</span>`).join(' ');
  }

  // Map a risk severity to a CSS dot class (CSS-only, not emoji).
  function sevClass(severity) {
    const s = String(severity || '').toLowerCase();
    if (s === 'high') return 'danger';
    if (s === 'medium') return 'warn';
    return 'ok';
  }
  // Human-readable label for a risk kind.
  function humanizeKind(kind) {
    const map = { cycle: 'Cycle', dead_export: 'Dead export', twin: 'Twin', hotspot: 'Hotspot' };
    if (map[kind]) return map[kind];
    const raw = String(kind || '');
    return raw ? raw.charAt(0).toUpperCase() + raw.slice(1) : '';
  }
  /** Short badge label for the pill state; unknown states read "Unknown", never blank. */
  function contextStateLabel(vm) {
    const map = {
      ready: 'Ready',
      'out-of-workspace': 'Outside workspace',
      'not-in-snapshot': 'Not indexed',
      'no-editor': 'No editor',
      'lsp-starting': 'Waiting',
      'lsp-stopped': 'Inactive',
      'lsp-error': 'Error',
    };
    return map[vm.state] || 'Unknown';
  }
  /** The Context + LSP badge pair shown in every ready header. */
  function lspBadgesHtml(vm) {
    const lsp = vm.lsp || { phase: 'stopped', label: 'Stopped' };
    return `<span class="badge context">Context: ${esc(contextStateLabel(vm))}</span>` +
      `<span class="badge lsp ${esc(lsp.phase)}">LSP: ${esc(lsp.label)}</span>`;
  }

  // Shared scope switcher markup — the escape hatch kept in every state.
  function switcherHtml() {
    return '<input id="scope" class="scope-switcher" placeholder="Inspect symbol or file…" />';
  }
  /** Bind the scope switcher's change event to a `rescope` message. */
  function wireSwitcher() {
    const s = document.getElementById('scope');
    if (s) s.addEventListener('change', (e) =>
      vscode.postMessage({ type: 'rescope', query: e.target.value }));
  }
  // Best-effort: does the string look like an absolute filesystem path? Used to
  // decide whether to show the muted path line in the out-of-workspace card.
  function looksLikePath(s) {
    return typeof s === 'string' && (s.startsWith('/') || /^[A-Za-z]:[\\/]/.test(s) || s.includes('/') || s.includes('\\'));
  }

  /** Empty state for a file outside every open project — shows no repo numbers,
   *  only the switcher and an "open a project file" affordance. */
  function renderOutOfWorkspace(vm) {
    const pathLine = looksLikePath(vm.file)
      ? `<p class="empty-path">${esc(vm.file)}</p>`
      : '';
    root.innerHTML = `
      <div class="empty-state">
        <h2 class="empty-head">Outside the indexed workspace</h2>
        ${pathLine}
        <p class="empty-body">This file is not part of any open Loctree project, so there is no structural context for it. Open a file from your project to see its blast radius, dependencies, and findings.</p>
        ${switcherHtml()}
        <button id="openproj" class="empty-action">Open a project file</button>
      </div>
    `;
    wireSwitcher();
    const btn = document.getElementById('openproj');
    if (btn) btn.addEventListener('click', () => vscode.postMessage({ type: 'openProjectFile' }));
  }

  /** Empty state when nothing is open; keeps the switcher so the pill is never a
   *  dead panel. */
  function renderNoEditor() {
    root.innerHTML = `
      <div class="empty-state">
        <h2 class="empty-head">No file selected</h2>
        <p class="empty-body">Open a file to see its Loctree context, or search below.</p>
        ${switcherHtml()}
      </div>
    `;
    wireSwitcher();
  }

  /** Empty state for an in-workspace file the snapshot has not indexed yet, with a
   *  scan affordance. */
  function renderNotInSnapshot(_vm) {
    root.innerHTML = `
      <div class="empty-state">
        <h2 class="empty-head">Not in the current snapshot</h2>
        <p class="empty-body">Loctree has not indexed this file yet. Scan the workspace to include it, then reopen this file for its context.</p>
        ${switcherHtml()}
        <button id="scanws" class="empty-action">Scan this workspace</button>
      </div>
    `;
    wireSwitcher();
    const btn = document.getElementById('scanws');
    if (btn) btn.addEventListener('click', () => vscode.postMessage({ type: 'scanWorkspace' }));
  }

  /** Card shown while the LSP is starting, stopped or failed. Surfaces the resolved
   *  binary and the server's own detail so a start failure is diagnosable here. */
  function renderLspUnavailable(vm) {
    const lsp = vm.lsp || { label: 'Stopped', message: 'Loctree LSP is not running.' };
    const title = vm.state === 'lsp-error'
      ? 'Loctree LSP failed'
      : vm.state === 'lsp-starting'
        ? 'Loctree LSP starting'
        : 'Loctree LSP inactive';
    const binary = lsp.serverCommand
      ? `<p class="empty-path">Binary: ${esc(lsp.serverCommand)}</p>`
      : '';
    const detail = lsp.detail
      ? `<p class="empty-path">${esc(lsp.detail)}</p>`
      : '';
    root.innerHTML = `
      <div class="empty-state">
        <h2 class="empty-head">${esc(title)}</h2>
        <p class="empty-body">${esc(lsp.message)}</p>
        ${binary}
        ${detail}
        ${switcherHtml()}
        <button id="scanws" class="empty-action">Initialize / Scan</button>
      </div>
    `;
    wireSwitcher();
    const btn = document.getElementById('scanws');
    if (btn) btn.addEventListener('click', () => vscode.postMessage({ type: 'scanWorkspace' }));
  }

  // ── Ready-render section builders. Each returns a string of HTML (possibly
  // empty); render() joins the non-empty ones so empty sections leave no shell.

  /** Header row: the scope label plus badges. LOC and blast-radius badges are
   *  scope-gated so a symbol pill never shows a file's numbers. */
  function headerHtml(vm) {
    const badges = [];
    badges.push(lspBadgesHtml(vm));
    // File LOC badge — only when we have a concrete file LOC (file scope).
    if (vm.fileLoc != null) {
      badges.push(`<span class="badge loc">${esc(vm.fileLoc)} LOC</span>`);
    }
    // Blast radius badge — scope-gated to file scope.
    if (vm.scope.kind === 'file') {
      badges.push(`<span class="badge blast">blast radius ${esc(vm.blastRadius.count)}</span>`);
    }
    const badgesHtml = badges.length ? `<span class="badges">${badges.join('')}</span>` : '';
    return `<header class="pill-head">
        <span class="file">${esc(vm.file || vm.scope.value)}</span>
        ${badgesHtml}
      </header>`;
  }

  /** WHAT IT DOES block; empty string when the server sent no summary. */
  function summaryHtml(vm) {
    if (!vm.summary) return '';
    return `<div class="label">WHAT IT DOES</div><p class="summary">${esc(vm.summary)}</p>`;
  }

  /** Risk-of-change band — states "safe to change" explicitly at zero consumers
   *  rather than rendering an empty section. */
  function blastBandHtml(vm) {
    if (vm.blastRadius.count === 0) {
      return `<section class="decision-band">
        <div class="label warn">BLAST RADIUS · RISK OF CHANGE</div>
        <p>No files depend on this yet — safe to change.</p>
      </section>`;
    }
    return `<section class="decision-band">
        <div class="label warn">BLAST RADIUS · RISK OF CHANGE</div>
        <p>A change here can touch ${esc(vm.blastRadius.count)} file(s).</p>
        <div>${tags(vm.blastRadius.direct, 'blast')}</div>
      </section>`;
  }

  /** Exports and dependencies columns; omits either column when it has no rows. */
  function structureColsHtml(vm) {
    const cols = [];
    if (vm.exports.length > 0) {
      cols.push(`<div><div class="label ok">DEFINES / EXPORTS · ${esc(vm.exports.length)}</div>${tags(vm.exports, 'export')}</div>`);
    }
    if (vm.deps.length > 0) {
      cols.push(`<div><div class="label dep">THIS FILE USES · ${esc(vm.deps.length)}</div>${tags(vm.deps, 'dep')}</div>`);
    }
    if (cols.length === 0) return '';
    return `<div class="cols">${cols.join('')}</div>`;
  }

  /** Findings for THIS file (max 5) with repo-wide totals kept on a separate muted
   *  line, so a reader never mistakes repo counts for file counts. */
  function fileRisksHtml(vm) {
    const rows = (vm.fileRisks || []).length
      ? vm.fileRisks.slice(0, 5).map((r) =>
          `<p><span class="dot ${sevClass(r.severity)}"></span>${esc(humanizeKind(r.kind))} — ${esc(r.message)}</p>`).join('')
      : '<p class="muted">No issues in this file</p>';
    const repo = `<p class="muted">Repo-wide: ${esc(vm.repoFindings.hotspots)} hotspots · ${esc(vm.repoFindings.dead)} dead · ${esc(vm.repoFindings.cycles)} cycles</p>`;
    return `<div class="findings"><div class="label danger">FINDINGS · THIS FILE</div>
          ${rows}
          ${repo}
        </div>`;
  }

  /** Bottom row: body preview (with a show-full-body button) beside the findings. */
  function bottomColsHtml(vm) {
    return `<div class="cols">
        <div class="body-preview"><div class="label">BODY PREVIEW</div>
          ${vm.bodyPreview && vm.bodyPreview.found ? `<pre>${esc(vm.bodyPreview.preview)}</pre><button id="full">show full body</button>` : '<p class="muted">—</p>'}
        </div>
        ${fileRisksHtml(vm)}
      </div>`;
  }

  // `suppressRepoHealth` omits the "Repo health: N/100" line — the literal pill
  // is a string-match surface with no repo-health context, so it must not print
  // a false "Repo health: 0/100".
  function footerHtml(vm, suppressRepoHealth) {
    const repoHealthLine = suppressRepoHealth
      ? ''
      : `<p class="muted">Repo health: ${esc(vm.repoHealth)}/100</p>`;
    return `<p class="muted">Snapshot: fresh · Agent pack: ${esc(vm.agentPackStatus)}</p>
      ${repoHealthLine}
      <div class="task"><input id="task" placeholder="task: inferred from WIP diff (editable)"/></div>
      <button id="cta" class="cta">Copy Agent Context</button>`;
  }

  // ── Literal scope (Task 6) ──────────────────────────────────────────────
  // A literal pill is a string-match surface: header + grouped occurrence list
  // + optional Load more. No blast/exports/body/findings (not applicable).

  // Keep the active literal query + groups in module state so Load-more can
  // append + dedup against what is already shown.
  let literalState = null; // { query, groups, hasMore, nextOffset, state, lsp, agentPackStatus, repoHealth }

  /** Literal occurrences as per-file groups of clickable line chips. */
  function occurrenceGroupsHtml(groups) {
    return (groups || []).map((g) => {
      const chips = (g.lines || []).map((ln) =>
        `<button class="occ-chip" data-file="${esc(g.file)}" data-line="${esc(ln)}">${esc(ln)}</button>`
      ).join(' ');
      return `<div class="occ-group">
          <div class="occ-file">${esc(g.file)} · ${esc((g.lines || []).length)}</div>
          <div class="occ-lines">${chips}</div>
        </div>`;
    }).join('');
  }

  /** Seed the module literal state from a fresh VM, then paint the occurrence pill. */
  function renderLiteral(vm) {
    literalState = {
      query: vm.scope.value,
      groups: vm.literalGroups || [],
      total: vm.literalTotal,
      hasMore: !!vm.literalHasMore,
      nextOffset: vm.literalNextOffset,
      state: vm.state,
      lsp: vm.lsp,
      agentPackStatus: vm.agentPackStatus,
      repoHealth: vm.repoHealth,
    };
    paintLiteral();
  }

  /** Paint the literal pill from module state. Load more appears only with a
   *  concrete next offset, so a bad contract cannot produce a button that refetches
   *  the same page forever. */
  function paintLiteral() {
    const st = literalState;
    // Gate Load more on a concrete next offset (not just hasMore): a contract
    // violation (has_more:true + next_offset:null) must NOT show a button that
    // would refetch the same page (offset:null → no advance).
    const canLoadMore = st.hasMore && st.nextOffset != null;
    const loadMore = canLoadMore
      ? '<button id="loadmore" class="load-more">Load more</button>'
      : '';
    root.innerHTML = `
      <header class="pill-head">
        <span class="file">"${esc(st.query)}" · ${esc(st.total != null ? st.total : countOccurrences(st.groups))} occurrences</span>
        <span class="badges">${lspBadgesHtml(st)}</span>
      </header>
      ${'<input id="scope" class="scope-switcher" placeholder="Inspect symbol or file…" />'}
      <div class="occurrences">${occurrenceGroupsHtml(st.groups)}</div>
      ${loadMore}
      ${footerHtml({ agentPackStatus: st.agentPackStatus, repoHealth: st.repoHealth }, true)}
    `;
    wireSwitcher();
    wireOccurrences();
    wireCta();
    const lm = document.getElementById('loadmore');
    if (lm) lm.addEventListener('click', () =>
      vscode.postMessage({ type: 'loadMoreLiteral', query: st.query, offset: st.nextOffset }));
  }

  /** Total lines currently rendered across all groups. */
  function countOccurrences(groups) {
    return (groups || []).reduce((n, g) => n + ((g.lines || []).length), 0);
  }

  /** Bind each occurrence chip to an `openFile` message carrying file + line. */
  function wireOccurrences() {
    root.querySelectorAll('.occ-chip').forEach((el) => {
      el.addEventListener('click', () => vscode.postMessage({
        type: 'openFile',
        file: el.getAttribute('data-file'),
        line: Number(el.getAttribute('data-line')),
      }));
    });
  }

  /** Bind the Copy Agent Context button, passing the editable task field along. */
  function wireCta() {
    const cta = document.getElementById('cta');
    if (cta) cta.addEventListener('click', () =>
      vscode.postMessage({ type: 'copyAgentContext', task: (document.getElementById('task') || {}).value || '' }));
  }

  // Merge an appended literal page into module state, de-duping (file, line).
  function mergeLiteral(incoming) {
    const order = [];
    const byFile = new Map();
    const absorb = (groups) => {
      (groups || []).forEach((g) => {
        if (!g || typeof g.file !== 'string') return;
        let set = byFile.get(g.file);
        if (!set) { set = new Set(); byFile.set(g.file, set); order.push(g.file); }
        (g.lines || []).forEach((ln) => { if (typeof ln === 'number') set.add(ln); });
      });
    };
    absorb(literalState.groups);
    absorb(incoming);
    literalState.groups = order.map((file) => ({
      file,
      lines: Array.from(byFile.get(file)).sort((a, b) => a - b),
    }));
  }

  // ── Scope-switcher resolution surfaces (Task 5) ─────────────────────────

  // Render an inline candidate list (many bodies) IN PLACE of the normal pill.
  function renderCandidates(query, items) {
    const rows = (items || []).map((it) =>
      `<button class="candidate" data-symbol="${esc(it.symbol)}" data-file="${esc(it.file)}">
         <span class="cand-symbol">${esc(it.symbol)}</span>
         <span class="cand-loc">${esc(it.file)}:${esc(it.line)}</span>
       </button>`
    ).join('');
    root.innerHTML = `
      <div class="resolution">
        <div class="label">MULTIPLE DEFINITIONS · ${esc((items || []).length)}</div>
        <p class="muted">"${esc(query)}" is defined in several places. Pick one:</p>
        <div class="candidate-list">${rows}</div>
        ${'<input id="scope" class="scope-switcher" placeholder="Inspect symbol or file…" />'}
      </div>
    `;
    wireSwitcher();
    root.querySelectorAll('.candidate').forEach((el) => {
      el.addEventListener('click', () => vscode.postMessage({
        type: 'rescopeCandidate',
        symbol: el.getAttribute('data-symbol'),
        file: el.getAttribute('data-file'),
      }));
    });
  }

  // Render the "no symbol found" prompt (zero bodies) with the literal count.
  function renderNoSymbol(query, literalCount) {
    root.innerHTML = `
      <div class="resolution">
        <p class="empty-body">No symbol found — ${esc(literalCount)} literal occurrence(s). Show as a pill?</p>
        <button id="asliteral" class="empty-action">Show as a pill</button>
        ${'<input id="scope" class="scope-switcher" placeholder="Inspect symbol or file…" />'}
      </div>
    `;
    wireSwitcher();
    const btn = document.getElementById('asliteral');
    if (btn) btn.addEventListener('click', () =>
      vscode.postMessage({ type: 'showAsLiteral', query: query }));
  }

  /** Top-level render. Dispatches empty/LSP/literal cards first, then composes the
   *  ready pill from the section builders, dropping any section that came back empty. */
  function render(vm) {
    if (vm.state === 'no-editor') { renderNoEditor(); return; }
    if (vm.state === 'lsp-starting' || vm.state === 'lsp-stopped' || vm.state === 'lsp-error') {
      renderLspUnavailable(vm);
      return;
    }
    if (vm.state === 'out-of-workspace') { renderOutOfWorkspace(vm); return; }
    if (vm.state === 'not-in-snapshot') { renderNotInSnapshot(vm); return; }
    if (vm.scope && vm.scope.kind === 'literal') { renderLiteral(vm); return; }
    literalState = null;
    const parts = [
      headerHtml(vm),
      '<input id="scope" class="scope-switcher" placeholder="Inspect symbol or file…" />',
      summaryHtml(vm),
      blastBandHtml(vm),
      structureColsHtml(vm),
      bottomColsHtml(vm),
      footerHtml(vm),
    ];
    root.innerHTML = parts.filter((p) => p !== '').join('\n');
    document.getElementById('scope').addEventListener('change', (e) =>
      vscode.postMessage({ type: 'rescope', query: e.target.value }));
    document.getElementById('cta').addEventListener('click', () =>
      vscode.postMessage({ type: 'copyAgentContext', task: (document.getElementById('task') || {}).value || '' }));
    const full = document.getElementById('full');
    if (full) full.addEventListener('click', () => vscode.postMessage({ type: 'showFullBody', symbol: vm.scope.value }));
  }

  /** Accept a host message only when it carries this pill's channel token; anything
   *  else is dropped without being read. */
  function trustedHostMessage(data) {
    const m = data;
    if (!m || typeof m !== 'object') return null;
    if (!loctreeChannel || m.loctreeChannel !== loctreeChannel) return null;
    return m;
  }

  window.addEventListener('message', (ev) => {
    if (ev.origin !== window.location.origin) {
      const origin = ev.origin;
      if (
        origin
        && origin !== 'null'
        && !/^vscode-webview:\/\/[A-Za-z0-9.-]+$/.test(origin)
        && !/^https:\/\/[A-Za-z0-9.+-]+\.vscode-cdn\.net$/.test(origin)
      ) return;
    }
    const m = trustedHostMessage(ev.data);
    if (!m) return;
    if (m.type === 'render') render(m.vm);
    if (m.type === 'candidates') renderCandidates(m.query, m.items);
    if (m.type === 'noSymbol') renderNoSymbol(m.query, m.literalCount);
    if (m.type === 'appendLiteral' && literalState) {
      mergeLiteral(m.groups);
      literalState.hasMore = !!m.hasMore;
      literalState.nextOffset = m.nextOffset;
      // After appending we know the rendered count; clear the cached total so
      // the header reflects what is actually shown (still a partial view).
      literalState.total = literalState.hasMore ? literalState.total : countOccurrences(literalState.groups);
      paintLiteral();
    }
    if (m.type === 'copied') {
      const cta = document.getElementById('cta');
      if (cta) { cta.textContent = 'Copied Agent Context'; cta.classList.add('copied'); }
    }
  });
})();
