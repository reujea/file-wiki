// ── Model (Tauri invoke) ──────────────────────────────────────
// Tauri 2.0: window.__TAURI_INTERNALS__.invoke
const invoke = (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke)
  ? window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__)
  : (window.__TAURI__ && window.__TAURI__.core ? window.__TAURI__.core.invoke : null);
const call = (cmd, args) => invoke ? invoke(cmd, args).catch(e => { console.error(`[${cmd}]`, e); return {}; }) : Promise.resolve({});

// ── 공통 모달 시스템 ──────────────────────────────────────────
const Modal = {
  _overlay: null,
  /**
   * 모달 팝업 열기
   * @param {string} title - 모달 제목
   * @param {string} bodyHtml - 모달 본문 HTML
   * @param {Object} opts - { onSave: async fn, saveLabel: string, wide: boolean }
   */
  open(title, bodyHtml, opts = {}) {
    const { onSave, saveLabel = '저장', wide = false } = opts;
    this.close();
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';
    overlay.innerHTML = `
      <div class="modal-container" style="${wide ? 'max-width:900px' : ''}">
        <div class="modal-header">
          <h3>${title}</h3>
          <button class="modal-close" data-modal-close>&times;</button>
        </div>
        <div class="modal-body">${bodyHtml}</div>
        <div class="modal-footer">
          <span class="modal-status" id="modal-status"></span>
          ${onSave ? `<button class="btn btn-secondary" data-modal-close>취소</button>
          <button class="btn btn-primary" data-modal-save>${saveLabel}</button>` :
          `<button class="btn btn-primary" data-modal-close>닫기</button>`}
        </div>
      </div>`;
    // 이벤트: 닫기
    overlay.querySelectorAll('[data-modal-close]').forEach(el =>
      el.addEventListener('click', () => this.close()));
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) this.close();
    });
    // 이벤트: 저장
    if (onSave) {
      const saveBtn = overlay.querySelector('[data-modal-save]');
      saveBtn.addEventListener('click', async () => {
        saveBtn.disabled = true;
        saveBtn.textContent = '저장 중...';
        try {
          await onSave(overlay);
          this.close();
        } catch (err) {
          const status = overlay.querySelector('#modal-status');
          if (status) { status.textContent = err.message || '오류 발생'; status.className = 'modal-status error'; }
          saveBtn.disabled = false;
          saveBtn.textContent = saveLabel;
        }
      });
    }
    // ESC 키
    this._escHandler = (e) => { if (e.key === 'Escape') this.close(); };
    document.addEventListener('keydown', this._escHandler);
    document.body.appendChild(overlay);
    this._overlay = overlay;
    // 첫 입력 요소 포커스
    const firstInput = overlay.querySelector('input, select, textarea');
    if (firstInput) setTimeout(() => firstInput.focus(), 50);
    return overlay;
  },
  close() {
    if (this._overlay) {
      this._overlay.remove();
      this._overlay = null;
    }
    if (this._escHandler) {
      document.removeEventListener('keydown', this._escHandler);
      this._escHandler = null;
    }
  },
  /** 모달 내부 상태 메시지 표시 */
  setStatus(msg, type = 'success') {
    const el = document.getElementById('modal-status');
    if (el) { el.textContent = msg; el.className = `modal-status ${type}`; }
  }
};

const API = {
  stats: () => call('get_stats'),
  search: (params) => call('search', { params }),
  documents: (params) => call('list_documents', { params }),
  document: (id) => call('get_document', { docId: id }),
  lintStrongClaims: () => call('get_lint_strong_claims'),
  // Phase 64 (lesson 19): health/lint/deleteDocument/fixBacklinks/kgPaths는 frontend 호출처 0건으로 정리됨
  listCredentials: () => call('list_credentials'),
  saveCredential: (cred) => call('save_credential', { credential: cred }),
  deleteCredential: (name) => call('delete_credential', { name }),
  kgNeighbors: (id) => call('kg_neighbors', { docId: id }),
  kgStats: () => call('kg_stats'),
  verificationMetrics: () => call('get_verification_metrics'),
  progress: () => call('get_progress'),
  queue: () => call('get_queue'),
  errors: () => call('get_errors'),
  todos: () => call('get_todos'),
  completeTodo: (todoId) => call('complete_todo', { todoId }),
  addTodo: (title, category, dueDate) => call('add_todo', { title, category, dueDate }),
  fileLog: (fileName, maxLines) => call('get_file_log', { fileName, maxLines }),
  getConfig: () => call('get_config'),
  updateConfig: (config) => call('save_config', { configJson: JSON.stringify(config) }),
  exportConfigToml: () => call('export_config_toml'),
  importConfigToml: (content) => call('import_config_toml', { tomlContent: content }),
  topics: () => call('list_topics'),
  topic: (path) => call('get_topic', { path }),
  updateTopic: (path, content) => call('update_topic', { path, content }),
  retryFailed: () => call('retry_failed'),
  // Phase 56: 단일 파이프라인 구조로 전환 — list/delete/reorder 백엔드 제거됨 (lesson 19)
  // Phase 64: savePipeline은 frontend 호출처 0건 (updateConfig가 파이프라인 저장 통합) → 정리됨
  rebuildEmbeddings: () => call('rebuild_embeddings'),
  rebuildAll: () => call('rebuild_all'),
  rebuildVectordb: () => call('rebuild_vectordb'),
  tokenUsage: () => call('get_token_usage'),
  simulatePipeline: (text) => call('simulate_pipeline', { inputText: text }),
  watcherStatus: () => call('get_watcher_status'),
  setWatcherActive: (active) => call('set_watcher_active', { active }),
  crossrefStats: () => call('get_crossref_stats'),
  hostTools: () => call('get_host_tools'),
  testHostTool: (tool) => call('test_host_tool', { tool }),
  getPrompts: () => call('get_prompts'),
  savePrompts: (content) => call('save_prompts', { content }),
  // Phase 73/74: 설정 도우미 (시나리오 기반 추천)
  setupReview: (scenario, userRole) => call('setup_review', { scenario, userRole }),
  setupReviewProfile: (profile) => call('setup_review', { profile }),
  setupApply: (scenario, acceptedPaths) => call('setup_apply', { scenario, acceptedPaths }),
  setupApplyProfile: (profile, acceptedPaths, applyCritical) => call('setup_apply', { profile, acceptedPaths, applyCritical: !!applyCritical }),
  // Phase 80
  setupModulesList: () => call('setup_modules_list'),
  setupApplyModules: (moduleIds, applyCritical, dryrun) => call('setup_apply_modules', { moduleIds, applyCritical: !!applyCritical, dryrun: !!dryrun }),
  getProcessingMetrics: () => call('get_processing_metrics'),
  getLlmCacheStats: () => call('get_llm_cache_stats'),
  clearLlmCache: () => call('clear_llm_cache'),
  gcLlmCacheNow: (maxEntries) => call('gc_llm_cache_now', { maxEntries: maxEntries || null }),
  c1ThresholdsList: () => call('c1_thresholds_list'),
  c1ThresholdSet: (key, value) => call('c1_threshold_set', { key, value }),
  piiPatternsList: () => call('pii_patterns_list'),
  piiPatternAdd: (name, pattern, enabled) => call('pii_pattern_add', { name, pattern, enabled }),
  piiPatternRemove: (name) => call('pii_pattern_remove', { name }),
  autoSuggestFromCounters: () => call('auto_suggest_from_counters'),
  // Phase 93 GUI 가시화 (Phase 91 A2 / 92 H1·H3·H5)
  anomalyReport: () => call('get_anomaly_report'),
  mcpToolCatalogFull: () => call('get_mcp_tool_catalog_full'),
  remoteStorageCapabilities: () => call('get_remote_storage_capabilities'),
  piiMaskConfig: () => call('get_pii_mask_config'),
  acceptSuggestedDecision: (decisionId) => call('accept_suggested_decision', { decisionId }),
  rejectSuggestedDecision: (decisionId) => call('reject_suggested_decision', { decisionId }),
  setupDecisionLogList: (limit) => call('setup_decision_log_list', { limit: limit || 50 }),
  setupSnapshotList: (limit) => call('setup_snapshot_list', { limit: limit || 20 }),
  setupSnapshotRollback: (snapshotId, reason) => call('setup_snapshot_rollback', { snapshotId, reason }),
  getSearchModeStats: () => call('get_search_mode_stats'),
  getCragStats: () => call('get_crag_stats'),
  getChunkStats: (sampleSize) => call('get_chunk_stats', { sampleSize: sampleSize || 50 }),
  // Phase 64: getRetentionConfig frontend 호출처 0건 → 정리됨
};

// ── Phase 66: 7탭 원복 + Pipeline 3탭 재구성 ────────────────────
// Pipeline 탭의 가운데 영역 3탭: [가공 파이프라인] [검색 파이프라인] [배치 설정]
const PIPELINE_TABS = [
  { id: 'process', label: '가공 파이프라인' },
  { id: 'search',  label: '검색 파이프라인' },
  { id: 'batch',   label: '배치 설정' },
];

// ── ViewModel (상태 + 렌더링) ─────────────────────────────────
const vm = {
  state: {
    stats: null,
    kgStats: null,
    searchResults: [],
    currentDoc: null,
    documents: [],
    activeTab: 'documents',
    config: null,
    configMeta: null,
    docPage: 1,
    docTotalPages: 1,
    topics: [],
    currentTopicPath: null,
    todos: null,
    verificationMetrics: null,
  },

  // Stats 카드 렌더링
  renderStats() {
    const s = this.state.stats || {};
    const kg = this.state.kgStats || {};
    document.getElementById('stat-total').textContent = s.total_documents || 0;
    // 처리 현황 (queue 데이터)
    const q = this.state._queueStats || {};
    const procEl = document.getElementById('stat-processing');
    const pendEl = document.getElementById('stat-pending');
    const failEl = document.getElementById('stat-failed');
    if (procEl) procEl.textContent = q.processing || 0;
    if (pendEl) pendEl.textContent = q.pending || 0;
    if (failEl) failEl.textContent = q.failed || 0;
    document.getElementById('stat-kg-nodes').textContent = kg.total_nodes || 0;
    document.getElementById('stat-kg-edges').textContent = kg.total_edges || 0;
    document.getElementById('stat-isolated').textContent = kg.isolated_nodes || 0;
    // Token usage (실시간 데이터)
    const tokIn = document.getElementById('stat-tokens-in');
    const tokOut = document.getElementById('stat-tokens-out');
    const apiCalls = document.getElementById('stat-api-calls');
    const tok = this.state._tokenUsage || {};
    if (tokIn) tokIn.textContent = tok.input_tokens != null ? tok.input_tokens.toLocaleString() : '-';
    if (tokOut) tokOut.textContent = tok.output_tokens != null ? tok.output_tokens.toLocaleString() : '-';
    if (apiCalls) apiCalls.textContent = tok.call_count != null ? tok.call_count.toLocaleString() : '-';
    // Ruflo A1: LLM 캐시
    const cache = this.state._llmCacheStats || {};
    const cEnt = document.getElementById('stat-llm-cache-entries');
    const cHits = document.getElementById('stat-llm-cache-hits');
    const cAvg = document.getElementById('stat-llm-cache-avg');
    const cGcAt = document.getElementById('stat-llm-cache-gc-at');
    const cGcDel = document.getElementById('stat-llm-cache-gc-deleted');
    if (cEnt) cEnt.textContent = cache.entries != null ? cache.entries.toLocaleString() : '-';
    if (cHits) cHits.textContent = cache.total_hits != null ? cache.total_hits.toLocaleString() : '-';
    if (cAvg) cAvg.textContent = cache.avg_hits_per_entry != null ? cache.avg_hits_per_entry.toFixed(2) : '-';
    // 마지막 GC 정보
    if (cGcAt) cGcAt.textContent = cache.last_gc?.at ? cache.last_gc.at.slice(0, 19).replace('T', ' ') : '-';
    if (cGcDel) cGcDel.textContent = cache.last_gc?.deleted != null ? cache.last_gc.deleted.toLocaleString() : '-';
  },

  // 문서 상세 렌더링
  renderDocDetail() {
    const panel = document.getElementById('doc-detail');
    const doc = this.state.currentDoc;
    if (!doc) { panel.classList.remove('active'); return; }
    panel.classList.add('active');
    document.getElementById('detail-title').textContent =
      (doc.doc_types || []).join(', ') + ' — ' + (doc.id || '').substring(0, 8);
    document.getElementById('detail-content').textContent = doc.content || '';
    const rels = doc.relations || [];
    document.getElementById('detail-relations').innerHTML = (rels.length
      ? '<b>관계:</b> ' + rels.map(r => `${r.type} → ${r.target.substring(0, 8)}`).join(', ')
      : '')
;
    // wikidocs 353407: 확인 필요 / 다시 물어볼 질문 노출 (값 비어있으면 영역 숨김)
    const aux = document.getElementById('detail-aux');
    if (aux) {
      const nv = doc.needs_verification || [];
      const oq = doc.open_questions || [];
      let html = '';
      if (nv.length) {
        html += '<div class="detail-aux-block"><b>확인 필요:</b><ul>'
          + nv.map(s => `<li>${this._escape(s)}</li>`).join('') + '</ul></div>';
      }
      if (oq.length) {
        html += '<div class="detail-aux-block"><b>다시 물어볼 질문:</b><ul>'
          + oq.map(s => `<li>${this._escape(s)}</li>`).join('') + '</ul></div>';
      }
      aux.innerHTML = html;
      aux.style.display = html ? '' : 'none';
    }
  },

  _escape(s) {
    return String(s)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  },

  // Phase 89 N-4: 강한 주장 lint 즉시 실행 (Verification 탭 카드)
  async runLintStrongClaims() {
    const panel = document.getElementById('lint-strong-claims-result');
    if (!panel) return;
    panel.innerHTML = '<div style="color:var(--color-text-muted);font-size:.8rem">실행 중...</div>';
    try {
      const data = await API.lintStrongClaims();
      const issues = data.issues || [];
      if (!issues.length) {
        panel.innerHTML = '<div style="color:var(--color-text-muted);font-size:.85rem">검출된 강한 주장이 없습니다.</div>';
        return;
      }
      const rows = issues.map(i =>
        `<tr><td style="padding:4px 8px;font-family:monospace;font-size:.75rem">${this._escape(i.doc_id.substring(0, 12))}</td>`
        + `<td style="padding:4px 8px;font-size:.8rem">${this._escape(i.description)}</td></tr>`
      ).join('');
      panel.innerHTML = `<div style="font-size:.8rem;margin-bottom:4px">총 ${issues.length}건</div>`
        + `<table style="width:100%;border-collapse:collapse">
            <thead><tr style="background:var(--color-surface-2)"><th style="text-align:left;padding:4px 8px;font-size:.75rem">문서</th><th style="text-align:left;padding:4px 8px;font-size:.75rem">검출 내용</th></tr></thead>
            <tbody>${rows}</tbody>
          </table>`;
    } catch (e) {
      panel.innerHTML = `<div style="color:var(--color-danger);font-size:.8rem">실패: ${this._escape(String(e))}</div>`;
    }
  },

  // KG 그래프 SVG 렌더링 (단일 문서 이웃 - detail용)
  renderKgGraph(docId, data) {
    // detail 패널에서 개별 문서 관계 표시 시 사용 (기존 호환)
    const svg = document.getElementById('kg-svg');
    if (!svg) return;
    // 전체 그래프가 이미 렌더링되어 있으면 개별 렌더링 건너뜀
    if (svg.dataset.fullGraph === 'true') return;
    const nodes = data.nodes || [];
    const edges = data.edges || [];
    if (!nodes.length) {
      svg.innerHTML = '<text x="50%" y="50%" fill="var(--color-text-dim)" text-anchor="middle" font-size="14">\uAD00\uACC4 \uC5C6\uC74C</text>';
      return;
    }
    let html = '';
    nodes.forEach((n, i) => {
      const x = 80 + i * 150;
      const y = 150;
      const isRoot = n.id === docId;
      html += `<circle cx="${x}" cy="${y}" r="20" fill="${isRoot ? 'var(--color-primary)' : 'var(--color-secondary)'}" opacity="0.8"/>`;
      html += `<text x="${x}" y="${y + 35}" fill="var(--color-text-muted)" text-anchor="middle" font-size="11">${n.doc_types[0] || '?'}</text>`;
      html += `<text x="${x}" y="${y + 48}" fill="var(--color-text-dim)" text-anchor="middle" font-size="9">${n.id.substring(0, 6)}</text>`;
    });
    edges.forEach(e => {
      const si = nodes.findIndex(n => n.id === e.source);
      const ti = nodes.findIndex(n => n.id === e.target);
      if (si >= 0 && ti >= 0) {
        html += `<line x1="${80 + si * 150}" y1="150" x2="${80 + ti * 150}" y2="150" stroke="var(--color-border)" stroke-width="2"/>`;
      }
    });
    svg.innerHTML = html;
  },

  // KG Ego Graph 렌더링 (선택 문서 중심 1-hop)
  async renderKGGraph(centerId) {
    const svg = document.getElementById('kg-svg');
    if (!svg) return;

    const docs = this.state.documents || [];
    // 중심 문서 결정: 인자 > 현재 선택 > 첫 문서
    const rootId = centerId || this.state.currentDoc?.id || (docs[0] && docs[0].id);
    if (!rootId) {
      svg.innerHTML = '<text x="50%" y="45%" text-anchor="middle" fill="var(--color-text-muted)" font-size="13">문서 목록에서 항목을 선택하면</text><text x="50%" y="55%" text-anchor="middle" fill="var(--color-text-muted)" font-size="13">관계 그래프가 표시됩니다</text>';
      svg.dataset.fullGraph = 'false';
      return;
    }

    // Ego graph: 중심 문서의 1-hop 이웃만 로드 (N+1 → 1회 호출)
    const nodes = [];
    const edges = [];
    const nodeMap = {};

    try {
      const data = await API.kgNeighbors(rootId);
      const kgNodes = data.nodes || [];
      const kgEdges = data.edges || [];

      for (const n of kgNodes) {
        if (!nodeMap[n.id]) {
          nodeMap[n.id] = nodes.length;
          nodes.push({
            id: n.id,
            label: n.id.substring(0, 20),
            type: 'document',
            docType: (n.doc_types || [])[0] || 'etc',
            degree: n.relation_count || 0,
            isRoot: n.id === rootId,
          });
        }
      }
      for (const e of kgEdges) {
        const si = nodeMap[e.source];
        const ti = nodeMap[e.target];
        if (si !== undefined && ti !== undefined) {
          edges.push({ source: si, target: ti, relType: e.relation || 'related_topic' });
        }
      }
    } catch(e) { /* skip */ }

    if (nodes.length === 0) {
      svg.innerHTML = '<text x="50%" y="50%" text-anchor="middle" fill="var(--color-text-muted)" font-size="14">\uBB38\uC11C\uB97C \uAC00\uACF5\uD558\uBA74 \uC9C0\uC2DD \uADF8\uB798\uD504\uAC00 \uD45C\uC2DC\uB429\uB2C8\uB2E4.</text>';
      svg.dataset.fullGraph = 'false';
      return;
    }

    const width = svg.clientWidth || 800;
    const height = 300;
    svg.setAttribute('viewBox', `0 0 ${width} ${height}`);

    // Initialize positions randomly
    for (const node of nodes) {
      node.x = width * 0.2 + Math.random() * width * 0.6;
      node.y = height * 0.2 + Math.random() * height * 0.6;
      node.vx = 0;
      node.vy = 0;
    }

    // Simple force simulation (50 iterations)
    for (let iter = 0; iter < 50; iter++) {
      // Repulsion between all nodes
      for (let i = 0; i < nodes.length; i++) {
        for (let j = i + 1; j < nodes.length; j++) {
          let dx = nodes[j].x - nodes[i].x;
          let dy = nodes[j].y - nodes[i].y;
          let dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
          let force = 5000 / (dist * dist);
          nodes[i].vx -= dx / dist * force;
          nodes[i].vy -= dy / dist * force;
          nodes[j].vx += dx / dist * force;
          nodes[j].vy += dy / dist * force;
        }
      }
      // Attraction along edges
      for (const edge of edges) {
        let s = nodes[edge.source], t = nodes[edge.target];
        let dx = t.x - s.x, dy = t.y - s.y;
        let dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
        let force = (dist - 100) * 0.01;
        s.vx += dx / dist * force;
        s.vy += dy / dist * force;
        t.vx -= dx / dist * force;
        t.vy -= dy / dist * force;
      }
      // Update positions
      for (const node of nodes) {
        node.x += node.vx * 0.5;
        node.y += node.vy * 0.5;
        node.vx *= 0.8;
        node.vy *= 0.8;
        node.x = Math.max(30, Math.min(width - 30, node.x));
        node.y = Math.max(30, Math.min(height - 30, node.y));
      }
    }

    // Color maps
    const typeColors = {
      meeting: '#38bdf8', log: '#4ade80', report: '#fb923c',
      study: '#a78bfa', memo: '#f472b6', todo: '#facc15',
      brainstorm: '#2dd4bf', etc: '#94a3b8',
    };
    const relColors = {
      references: '#64748b', updates: '#38bdf8',
      related_topic: '#4ade80', supersedes: '#ef4444',
    };

    // Render SVG
    let html = '';

    // Edges
    for (const edge of edges) {
      const s = nodes[edge.source], t = nodes[edge.target];
      const color = relColors[edge.relType] || '#64748b';
      html += `<line x1="${s.x}" y1="${s.y}" x2="${t.x}" y2="${t.y}" stroke="${color}" stroke-width="1.5" stroke-opacity="0.6"/>`;
    }

    // Nodes
    for (const node of nodes) {
      const r = Math.max(6, Math.min(20, 6 + node.degree * 2));
      const color = typeColors[node.docType] || typeColors.etc;
      html += `<circle cx="${node.x}" cy="${node.y}" r="${r}" fill="${color}" fill-opacity="0.8" stroke="${color}" stroke-width="1.5" cursor="pointer">`;
      html += `<title>${node.label} (${node.docType}, \uC5F0\uACB0: ${node.degree})</title>`;
      html += `</circle>`;
      if (r >= 10) {
        html += `<text x="${node.x}" y="${node.y + r + 12}" text-anchor="middle" fill="var(--color-text-dim)" font-size="9">${node.label.substring(0, 15)}</text>`;
      }
    }

    // Legend
    html += `<g transform="translate(10, ${height - 60})">`;
    let ly = 0;
    for (const [type, color] of Object.entries(typeColors).slice(0, 6)) {
      html += `<circle cx="6" cy="${ly}" r="4" fill="${color}"/>`;
      html += `<text x="14" y="${ly + 3}" fill="var(--color-text-dim)" font-size="8">${type}</text>`;
      ly += 12;
    }
    html += `</g>`;

    svg.innerHTML = html;
    svg.dataset.fullGraph = 'true';
  },

  // 문서 테이블 렌더링
  renderDocList() {
    const tbody = document.getElementById('doc-table-body');
    const docs = this.state.documents;
    if (!docs.length) {
      tbody.innerHTML = `<tr><td colspan="6" style="text-align:center;padding:var(--spacing-xl)">
        <div style="color:var(--color-text-muted);font-size:.9rem;margin-bottom:var(--spacing-md)">아직 가공된 문서가 없습니다</div>
        <div style="text-align:left;display:inline-block;font-size:.82rem;color:var(--color-text-dim);line-height:2">
          <strong>시작하기:</strong><br>
          1. <strong>Settings</strong> 탭에서 크레덴셜(Claude CLI 등)을 등록하세요<br>
          2. <strong>Pipeline</strong> 탭에서 가공 설정을 확인하세요<br>
          3. <code style="background:var(--color-bg);padding:2px 6px;border-radius:3px">inbox/</code> 폴더에 문서를 넣으면 자동으로 가공됩니다
        </div>
      </td></tr>`;
      return;
    }
    tbody.innerHTML = docs.map(d => {
      // Phase 61 G1: hierarchy(상위 제목 계층)를 breadcrumb으로 표시
      const hierarchy = (d.hierarchy && d.hierarchy.length)
        ? d.hierarchy.map(h => `<span class="crumb">${h}</span>`).join('<span class="sep"> › </span>')
        : '<span style="color:var(--color-text-dim)">-</span>';
      return `<tr data-action="show-doc" data-id="${d.id}">
        <td class="doc-id">${d.id.substring(0, 10)}</td>
        <td><span class="doc-type">${(d.doc_types || []).join(', ')}</span></td>
        <td>${d.date || ''}</td>
        <td>${d.topic || '-'}</td>
        <td class="hierarchy">${hierarchy}</td>
        <td>${d.access_count != null ? d.access_count : '-'}</td>
      </tr>`;
    }).join('');
    const pag = document.getElementById('doc-pagination');
    if (this.state.docTotalPages > 1) {
      pag.innerHTML = `<div class="pagination">
        <button class="btn btn-secondary" data-action="prev-page" ${this.state.docPage <= 1 ? 'disabled' : ''}>이전</button>
        <span style="color:var(--color-text-muted)">${this.state.docPage} / ${this.state.docTotalPages}</span>
        <button class="btn btn-secondary" data-action="next-page" ${this.state.docPage >= this.state.docTotalPages ? 'disabled' : ''}>다음</button>
      </div>`;
    } else { pag.innerHTML = ''; }
  },

  // 탭 전환 (Phase 66: 7탭 원복)
  switchTab(tab) {
    // Pipeline 탭에서 나갈 때 미저장 변경사항 즉시 flush
    if (this.state.activeTab === 'pipeline' && this._pbAutoSaveTimer) {
      clearTimeout(this._pbAutoSaveTimer);
      this._pbAutoSaveTimer = null;
      this._pbSave();
    }
    this.state.activeTab = tab;
    Modal.close();
    document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.dataset.tab === tab));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.toggle('active', c.id === 'tab-' + tab));
    if (tab === 'documents' && !this.state.documents.length) this.loadDocuments();
    if (tab === 'processing') {
      this.loadProcessing();
      // 검증 영역 통합 (이전 Verification 탭)
      if (!this.state.verificationMetrics) this.loadVerificationMetrics();
      this.loadAnomalyReport();
    }
    if (tab === 'todos' && !this.state.todos) this.loadTodos();
    if (tab === 'topics' && !this.state.topics.length) this.loadTopics();
    if (tab === 'pipeline') this.loadPipelineBuilder();
    if (tab === 'settings' && (!this.state.config || !this.state.configMeta)) this.loadSettings();
    if (tab === 'settings') { this.loadPiiMaskToggle(); this.loadMcpCatalog(); }
    if (tab === 'settings') {
      this.loadDecisionLog();
      this.loadC1Thresholds();
      this.loadPiiPatterns();
    }
  },

  // Phase 80-1: 설정 도우미 진입점 — 3분기 (일반 / AI 분석 / 직접 모듈 선택)
  openSetupAssistant() {
    const html = `
      <div style="margin-bottom:var(--spacing-md);padding-bottom:var(--spacing-sm);border-bottom:1px dashed var(--color-border)">
        <h4 style="color:var(--color-primary);margin-bottom:6px">설정 진입 방식 선택</h4>
        <p style="color:var(--color-text-muted);font-size:.78rem;margin:0 0 10px">
          처음이라면 <strong>일반 설정으로 시작</strong>하고 50~200파일 처리 후 AI에 분석 요청을 권장합니다.
        </p>
        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px">
          <button class="btn btn-secondary" data-action="setup-quickstart-general" style="text-align:left;padding:12px;display:flex;flex-direction:column;align-items:flex-start">
            <span style="font-weight:600">⚡ 일반 설정으로 시작</span>
            <span style="color:var(--color-text-dim);font-size:.7rem;margin-top:4px">기본값 그대로 즉시 사용. 변경 없음.</span>
          </button>
          <button class="btn btn-primary" data-action="setup-mcp-flow" style="text-align:left;padding:12px;display:flex;flex-direction:column;align-items:flex-start">
            <span style="font-weight:600">🤖 AI에게 분석 요청</span>
            <span style="color:var(--color-text-dim);font-size:.7rem;margin-top:4px">Claude Code MCP — 코퍼스 패턴 기반 추천 (50파일+ 권장)</span>
          </button>
          <button class="btn btn-secondary" data-action="open-setup-modules" style="text-align:left;padding:12px;display:flex;flex-direction:column;align-items:flex-start">
            <span style="font-weight:600">🧩 직접 동작 모듈 선택</span>
            <span style="color:var(--color-text-dim);font-size:.7rem;margin-top:4px">12개 모듈에서 원하는 동작을 직접 체크</span>
          </button>
        </div>
        <div style="margin-top:8px;text-align:right">
          <button class="btn" data-action="open-setup-fallback" style="font-size:.7rem;color:var(--color-text-dim);background:none;border:none;cursor:pointer;padding:4px 8px" title="legacy: 5축 프로파일 폼 (Phase 76)">
            (고급) 5축 프로파일 폼 ▸
          </button>
        </div>
      </div>

      <div class="settings-section" style="margin-bottom:var(--spacing-md)">
        <h4 style="color:var(--color-primary);margin-bottom:var(--spacing-sm)">1. MCP 서버 진입</h4>
        <p style="color:var(--color-text-muted);font-size:.82rem;margin-bottom:var(--spacing-sm)">
          다음 명령으로 stdio MCP 서버를 시작합니다:
        </p>
        <pre style="background:var(--color-bg);border:1px solid var(--color-border);padding:.5rem;border-radius:4px;font-family:var(--font-mono);font-size:.78rem;color:var(--color-success);overflow-x:auto;margin:0">file-pipeline-tauri.exe serve
# 또는
pipeline.exe serve</pre>
      </div>

      <div class="settings-section" style="margin-bottom:var(--spacing-md)">
        <h4 style="color:var(--color-primary);margin-bottom:var(--spacing-sm)">2. MCP 연계</h4>
        <p style="color:var(--color-text-muted);font-size:.82rem;line-height:1.6;margin-bottom:var(--spacing-sm)">
          <strong>방법 A — CLI 명령</strong> (가장 빠름):
        </p>
        <pre style="background:var(--color-bg);border:1px solid var(--color-border);padding:.5rem;border-radius:4px;font-family:var(--font-mono);font-size:.74rem;color:var(--color-success);overflow-x:auto;margin:0 0 var(--spacing-sm) 0"># Claude Code CLI에 MCP 서버 등록
claude mcp add file-pipeline "C:\\\\path\\\\to\\\\file-pipeline-tauri.exe" serve

# scope 옵션 (--scope user / project / local, 기본 local)
claude mcp add --scope user file-pipeline "C:\\\\path\\\\to\\\\file-pipeline-tauri.exe" serve

# 등록 확인
claude mcp list

# 삭제
claude mcp remove file-pipeline</pre>

        <p style="color:var(--color-text-muted);font-size:.82rem;line-height:1.6;margin:var(--spacing-sm) 0">
          <strong>방법 B — JSON 직접 편집</strong>:
        </p>
        <p style="color:var(--color-text-dim);font-size:.74rem;line-height:1.5;margin:0 0 6px 0">
          • <strong>local</strong> (현재 프로젝트, 자동 등록): <code>.mcp.json</code><br>
          • <strong>project</strong> (팀 공유, git 커밋): <code>.claude/mcp.json</code><br>
          • <strong>user</strong> (전 프로젝트): <code>~/.claude.json</code> (Windows: <code>%USERPROFILE%\\.claude.json</code>)
        </p>
        <pre style="background:var(--color-bg);border:1px solid var(--color-border);padding:.5rem;border-radius:4px;font-family:var(--font-mono);font-size:.72rem;color:var(--color-text-muted);overflow-x:auto;margin:0">{
  "mcpServers": {
    "file-pipeline": {
      "command": "C:\\\\path\\\\to\\\\file-pipeline-tauri.exe",
      "args": ["serve"]
    }
  }
}</pre>

        <p style="color:var(--color-text-muted);font-size:.82rem;line-height:1.6;margin:var(--spacing-sm) 0 6px">
          <strong>scope 샘플</strong>:
        </p>
        <table style="width:100%;border-collapse:collapse;font-size:.74rem">
          <thead>
            <tr style="border-bottom:1px solid var(--color-border)">
              <th style="text-align:left;padding:6px;color:var(--color-text-muted)">scope</th>
              <th style="text-align:left;padding:6px;color:var(--color-text-muted)">위치</th>
              <th style="text-align:left;padding:6px;color:var(--color-text-muted)">언제 사용</th>
            </tr>
          </thead>
          <tbody>
            <tr style="border-bottom:1px dashed var(--color-border)">
              <td style="padding:6px;font-family:var(--font-mono);color:var(--color-success)">local</td>
              <td style="padding:6px;color:var(--color-text-dim)"><code>.mcp.json</code></td>
              <td style="padding:6px;color:var(--color-text-muted)">현재 디렉토리에서만, 빠른 시작용</td>
            </tr>
            <tr style="border-bottom:1px dashed var(--color-border)">
              <td style="padding:6px;font-family:var(--font-mono);color:var(--color-secondary)">project</td>
              <td style="padding:6px;color:var(--color-text-dim)"><code>.claude/mcp.json</code></td>
              <td style="padding:6px;color:var(--color-text-muted)">팀과 git으로 공유</td>
            </tr>
            <tr>
              <td style="padding:6px;font-family:var(--font-mono);color:var(--color-warning)">user</td>
              <td style="padding:6px;color:var(--color-text-dim)"><code>~/.claude.json</code></td>
              <td style="padding:6px;color:var(--color-text-muted)">모든 프로젝트에서 자동 인식 (권장)</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="settings-section" style="margin-bottom:var(--spacing-md)">
        <h4 style="color:var(--color-primary);margin-bottom:var(--spacing-sm)">3. AI와 자연어로 리뷰</h4>
        <p style="color:var(--color-text-muted);font-size:.82rem;line-height:1.6;margin-bottom:var(--spacing-sm)">
          Claude Code에서 다음과 같이 입력하세요. AI가 <code>setup_review</code> 도구를 호출해 추천을 제시하고, 승인하면 <code>setup_apply</code>로 자동 반영합니다.
        </p>

        <p style="color:var(--color-text-muted);font-size:.78rem;line-height:1.6;margin:0 0 4px">방법 A — 한 줄 시나리오 (가장 빠름)</p>
        <div style="display:flex;flex-direction:column;gap:6px;margin-bottom:var(--spacing-sm)">
          <code style="background:rgba(56,189,248,0.1);padding:6px 10px;border-radius:4px;font-size:.78rem">"pipeline 설정 하자 — 회의록 위주야"</code>
          <code style="background:rgba(56,189,248,0.1);padding:6px 10px;border-radius:4px;font-size:.78rem">"연구 논문이 많아 — 검색 정밀도 위주로 조정해줘"</code>
          <code style="background:rgba(56,189,248,0.1);padding:6px 10px;border-radius:4px;font-size:.78rem">"코드 저장소를 정리할 거야. 민감 키 격리도 강화해줘"</code>
        </div>

        <p style="color:var(--color-text-muted);font-size:.78rem;line-height:1.6;margin:var(--spacing-sm) 0 4px">방법 B — 5축 상세 (정확한 추천)</p>
        <p style="color:var(--color-text-dim);font-size:.74rem;line-height:1.5;margin:0 0 6px">
          AI가 5축을 묻고 답변에 따라 가공/검색 파이프라인 노드별 변경 사항을 제시합니다. 답변 양식 예시:
        </p>
        <pre style="background:var(--color-bg);border:1px solid var(--color-border);padding:.5rem;border-radius:4px;font-family:var(--font-mono);font-size:.74rem;color:var(--color-text-muted);overflow-x:auto;margin:0 0 var(--spacing-sm) 0">문서:    코드, 회의록, 연구자료, 일반, 할 일 목록
민감도:   중간
처리량:   보통
검색 목적: 탐색, 관련 문서 확인
협업 형태: 혼자</pre>
        <p style="color:var(--color-text-dim);font-size:.72rem;line-height:1.5;margin:6px 0 0">
          AI가 받은 정보를 <code>setup_review</code>의 <code>profile</code> 인자로 변환해 호출합니다. 응답에는 가공 파이프라인 영향 노드 / 검색 파이프라인 영향 노드 / 위험도(low~critical) / 충돌 해소 결과가 포함됩니다.
        </p>
      </div>

      <div class="settings-section" style="margin-bottom:var(--spacing-md)">
        <h4 style="color:var(--color-primary);margin-bottom:var(--spacing-sm)">4. AI 리뷰 가이드 (사용자 기준)</h4>
        <p style="color:var(--color-text-muted);font-size:.78rem;line-height:1.6;margin-bottom:var(--spacing-sm)">
          AI가 추천을 검토할 때 다음 관점을 우선 적용하도록 요청하세요.
        </p>
        <ul style="color:var(--color-text-muted);font-size:.78rem;line-height:1.7;margin:0;padding-left:1.2rem">
          <li>가공/검색 파이프라인 어느 단계가 영향을 받는지 노드 단위로 표시</li>
          <li>P0 필수 / P1 권장 / P2 선택을 분리해 우선순위 제시</li>
          <li>위험도 critical (예: <code>retention.enabled</code>)은 별도 경고 + 명시 동의 후에만 적용</li>
          <li>현재 값과 추천 값 모두 보여주고, 변경의 정량 영향(예: 청크 ROUGE-L 개선 여지)을 1~2문장 설명</li>
          <li>적용 후 <code>setup_snapshot_measure</code>로 효과 측정 + 자동 롤백 권고 받기</li>
        </ul>
      </div>

      <div class="settings-section">
        <h4 style="color:var(--color-primary);margin-bottom:var(--spacing-sm)">등록된 MCP 도구 (총 13개)</h4>
        <p style="color:var(--color-text-muted);font-size:.78rem;line-height:1.6">
          <code>setup_review</code> · <code>setup_apply</code> — 설정 리뷰<br>
          <code>search</code> · <code>get_document</code> · <code>list_documents</code> · <code>stats</code> · <code>lint</code> — 검색·통계<br>
          <code>kg_neighbors</code> · <code>kg_paths</code> · <code>kg_stats</code> — 지식 그래프<br>
          <code>list_todos</code> · <code>complete_todo</code> · <code>revise_topic</code> — 작업·토픽
        </p>
      </div>
    `;
    Modal.open('🤖 AI 설정 도우미 (MCP)', html, { wide: true });
  },

  // Phase 80-1: 일반 설정 시작 (변경 없음, 안내만)
  setupQuickstartGeneral() {
    const html = `
      <div style="padding:var(--spacing-md)">
        <h4 style="color:var(--color-primary);margin-bottom:var(--spacing-sm)">⚡ 일반 설정으로 시작 완료</h4>
        <p style="color:var(--color-text-muted);font-size:.85rem;line-height:1.6;margin-bottom:var(--spacing-sm)">
          현재 <code>pipeline.toml</code>의 기본값을 그대로 사용합니다. 변경된 설정은 없습니다.
        </p>
        <div class="settings-section" style="margin-top:var(--spacing-md)">
          <h5 style="color:var(--color-text);margin:0 0 8px;font-size:.85rem">권장 다음 단계</h5>
          <ol style="color:var(--color-text-muted);font-size:.78rem;line-height:1.7;margin:0;padding-left:1.4rem">
            <li><code>inbox/</code>에 파일을 넣어 50~200파일 처리</li>
            <li>이 모달을 다시 열어 <strong>"🤖 AI에게 분석 요청"</strong> 또는 <strong>"🧩 직접 동작 모듈 선택"</strong></li>
            <li>실제 사용 패턴 기반으로 정밀 조정</li>
          </ol>
        </div>
      </div>`;
    Modal.open('⚡ 일반 설정 시작', html);
  },

  // Phase 80-1: 직접 동작 모듈 선택
  async openSetupModules() {
    Modal.open('🧩 동작 모듈 선택', '<p style="padding:var(--spacing-md);color:var(--color-text-muted)">로딩 중...</p>', { wide: true });
    try {
      const data = await API.setupModulesList();
      const modules = data.modules || [];
      this._renderSetupModulesForm(modules);
    } catch(e) {
      const body = document.querySelector('.modal-body');
      if (body) body.innerHTML = `<p style="color:var(--color-error);padding:var(--spacing-md)">로딩 실패: ${e}</p>`;
    }
  },

  _renderSetupModulesForm(modules) {
    const body = document.querySelector('.modal-body');
    if (!body) return;
    const groups = {
      process: { label: '📥 가공 측면', items: [] },
      search:  { label: '🔍 검색 측면', items: [] },
      ops:     { label: '⚙ 운영 측면',   items: [] },
    };
    modules.forEach(m => {
      if (groups[m.group]) groups[m.group].items.push(m);
    });

    let html = `
      <div style="margin-bottom:var(--spacing-md);padding-bottom:var(--spacing-sm);border-bottom:1px dashed var(--color-border)">
        <p style="color:var(--color-text-muted);font-size:.82rem;margin:0">
          원하는 동작을 선택하세요. 같은 그룹의 배타 모듈은 하나만 골라야 합니다.
          충돌 시 보수적 값(큰 청크/활성화/합집합)으로 자동 해소됩니다.
        </p>
      </div>`;

    Object.entries(groups).forEach(([gid, g]) => {
      if (!g.items.length) return;
      html += `<div class="settings-section" style="margin-bottom:var(--spacing-sm)">
        <h4 style="margin:0 0 8px;color:var(--color-primary);font-size:.9rem">${g.label}</h4>`;
      g.items.forEach(m => {
        const exclTag = m.exclusive_group
          ? `<span style="background:var(--color-warning);color:#0f172a;font-size:.62rem;padding:1px 5px;border-radius:3px;margin-left:4px">배타</span>`
          : '';
        const paths = (m.paths || []).join(', ');
        html += `
          <label style="display:flex;align-items:flex-start;gap:8px;padding:6px 0;border-bottom:1px dashed var(--color-border)">
            <input type="checkbox" data-module-id="${m.id}" data-exclusive-group="${m.exclusive_group || ''}" style="margin-top:3px">
            <div style="flex:1">
              <div style="font-size:.85rem">${m.icon} <strong>${this._escHtml(m.label)}</strong>${exclTag}
                <span style="color:var(--color-text-dim);font-size:.7rem;margin-left:4px">(${m.change_count}건)</span>
              </div>
              <div style="color:var(--color-text-dim);font-size:.72rem;margin-top:2px">${this._escHtml(m.hint)}</div>
              <div style="color:var(--color-text-dim);font-size:.68rem;margin-top:2px;font-family:var(--font-mono)">${this._escHtml(paths)}</div>
            </div>
          </label>`;
      });
      html += `</div>`;
    });

    html += `
      <div style="display:flex;justify-content:space-between;align-items:center;margin-top:var(--spacing-md);padding-top:var(--spacing-sm);border-top:1px dashed var(--color-border)">
        <label style="font-size:.78rem;color:var(--color-text-muted)">
          <input type="checkbox" id="modules-apply-critical"> Critical 항목 적용 동의
        </label>
        <div>
          <span id="modules-status" style="font-size:.72rem;color:var(--color-text-dim);margin-right:var(--spacing-sm)"></span>
          <button class="btn btn-secondary" data-action="modules-dryrun" style="margin-right:6px">미리보기 (dryrun)</button>
          <button class="btn btn-primary" data-action="modules-apply">적용</button>
        </div>
      </div>
      <div id="modules-result" style="margin-top:var(--spacing-md)"></div>
    `;
    body.innerHTML = html;

    // 배타 그룹 — 같은 그룹 체크 시 다른 항목 자동 해제
    body.querySelectorAll('input[type=checkbox][data-module-id]').forEach(cb => {
      cb.addEventListener('change', () => {
        const eg = cb.dataset.exclusiveGroup;
        if (!eg || !cb.checked) return;
        body.querySelectorAll(`input[type=checkbox][data-exclusive-group="${eg}"]`).forEach(other => {
          if (other !== cb) other.checked = false;
        });
      });
    });
  },

  _readSelectedModuleIds() {
    const body = document.querySelector('.modal-body');
    if (!body) return [];
    return Array.from(body.querySelectorAll('input[type=checkbox][data-module-id]'))
      .filter(cb => cb.checked)
      .map(cb => cb.dataset.moduleId);
  },

  async _runModulesAction(dryrun) {
    const ids = this._readSelectedModuleIds();
    const status = document.getElementById('modules-status');
    if (ids.length === 0) {
      if (status) { status.textContent = '모듈을 1개 이상 선택하세요'; status.style.color = 'var(--color-warning)'; }
      return;
    }
    const applyCritical = !!document.getElementById('modules-apply-critical')?.checked;
    if (status) { status.textContent = dryrun ? '미리보기 산출 중...' : '적용 중...'; status.style.color = 'var(--color-text-dim)'; }
    try {
      const res = await API.setupApplyModules(ids, applyCritical, dryrun);
      const result = document.getElementById('modules-result');
      if (dryrun) {
        const changes = res.changes || [];
        if (status) { status.textContent = `미리보기: ${changes.length}건 변경 예상`; status.style.color = 'var(--color-success)'; }
        if (result) result.innerHTML = this._renderModuleChangesPreview(changes);
      } else {
        const applied = res.applied || [];
        if (status) { status.textContent = `✓ ${applied.length}건 적용. 백업: ${res.backup}`; status.style.color = 'var(--color-success)'; }
        this.state.config = null;
        this.state.configMeta = null;
        if (this.state.activeTab === 'pipeline') this.loadPipelineBuilder();
        if (this.state.activeTab === 'settings') this.loadSettings();
      }
    } catch(e) {
      if (status) { status.textContent = '실패: ' + e; status.style.color = 'var(--color-error)'; }
    }
  },

  _renderModuleChangesPreview(changes) {
    if (!changes.length) return '<p style="color:var(--color-text-muted);padding:var(--spacing-sm)">변경 없음 — 현재 설정이 이미 모듈 추천과 일치</p>';
    let rows = '';
    changes.forEach(c => {
      const conflict = c.conflict_note
        ? `<div style="color:var(--color-warning);font-size:.7rem;margin-top:2px">⚠ ${this._escHtml(c.conflict_note)}</div>`
        : '';
      rows += `<tr>
        <td style="padding:6px"><code>${this._escHtml(c.path)}</code><div style="color:var(--color-text-dim);font-size:.7rem;margin-top:2px">${this._escHtml(c.reason)}</div>${conflict}</td>
        <td style="padding:6px;color:var(--color-text-dim);max-width:140px;word-break:break-all">${this._escHtml(JSON.stringify(c.current))}</td>
        <td style="padding:6px;color:var(--color-success);max-width:140px;word-break:break-all">${this._escHtml(JSON.stringify(c.recommended))}</td>
      </tr>`;
    });
    return `<table style="width:100%;border-collapse:collapse;font-size:.78rem">
      <thead><tr style="border-bottom:1px solid var(--color-border)">
        <th style="text-align:left;padding:6px">path / 이유</th>
        <th style="text-align:left;padding:6px;width:140px">현재</th>
        <th style="text-align:left;padding:6px;width:140px">추천</th>
      </tr></thead>
      <tbody>${rows}</tbody>
    </table>`;
  },

  // Phase 75: Tauri 단독 폴백 — Phase 74의 정적 룰 모달
  openSetupFallback() {
    // Phase 76: 5축 프로파일 폼 (외부 전문가 권장안)
    // Phase 79: 프리셋 카드(일반/전문/회의/코드/팀) + 모달 selector 버그 수정
    const contentTypes = [
      { id: 'meeting',  label: '회의록' },
      { id: 'research', label: '연구·논문' },
      { id: 'code',     label: '코드·기술' },
      { id: 'legal',    label: '법무·계약' },
      { id: 'general',  label: '일반' },
    ];
    const sliders = contentTypes.map(c => `
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px">
        <label style="width:90px;font-size:.78rem">${c.label}</label>
        <input type="range" min="0" max="100" value="${c.id === 'general' ? 100 : 0}" data-axis="content" data-kind="${c.id}" style="flex:1" oninput="this.nextElementSibling.textContent=this.value+'%'">
        <span style="width:42px;text-align:right;font-size:.78rem;color:var(--color-text-dim)">${c.id === 'general' ? 100 : 0}%</span>
      </div>`).join('');

    const axisRow = (label, name, options) => `
      <div style="display:flex;align-items:center;gap:12px;margin-bottom:8px">
        <label style="width:120px;font-size:.82rem">${label}</label>
        <select data-axis="${name}" style="flex:1;padding:6px;background:var(--color-surface);border:1px solid var(--color-border);border-radius:4px;color:var(--color-text)">
          ${options.map(o => `<option value="${o.id}">${o.label}</option>`).join('')}
        </select>
      </div>`;

    // 프리셋 카드 — 클릭 시 5축 폼 일괄 채움
    const presets = [
      { id:'general',  icon:'📂', label:'일반',     hint:'특정 도메인 없이 일반 문서' },
      { id:'expert',   icon:'🎓', label:'전문가',   hint:'연구·논문 정밀 검색 + 장기 보존' },
      { id:'meeting',  icon:'📋', label:'회의록',   hint:'회의·결정·액션 추적' },
      { id:'code',     icon:'💻', label:'코드',     hint:'코드 스니펫·민감 키 격리' },
      { id:'secure',   icon:'🔒', label:'보안',     hint:'민감도 high + 암호화 + warn 로그' },
      { id:'team',     icon:'👥', label:'팀 협업',  hint:'팀 규모 + 빈번한 lint' },
      { id:'allround', icon:'🧰', label:'올라운드', hint:'코드+회의+연구+일반 균등 / 탐색' },
    ];
    const presetCards = presets.map(p => `
      <button class="btn btn-secondary" data-action="setup-preset-pick" data-preset="${p.id}"
        style="text-align:left;padding:10px;display:flex;flex-direction:column;align-items:flex-start;font-size:.78rem;line-height:1.3">
        <span style="font-weight:600">${p.icon} ${p.label}</span>
        <span style="color:var(--color-text-dim);font-size:.7rem;margin-top:3px">${p.hint}</span>
      </button>`).join('');

    const html = `
      <div style="margin-bottom:var(--spacing-md)">
        <p style="color:var(--color-text-muted);font-size:.85rem">
          프리셋을 고르거나 5개 축으로 직접 입력하세요. 룰 엔진이 추천 설정을 제안하고, 적용 전에 검토 가능. 자동 백업(.bak) 생성.
        </p>
      </div>

      <div class="settings-section" style="margin-bottom:var(--spacing-sm)">
        <h4 style="margin:0 0 8px 0;color:var(--color-primary);font-size:.9rem">⚡ 프리셋 (클릭 시 폼 자동 채움)</h4>
        <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:6px">${presetCards}</div>
      </div>

      <div class="settings-section" style="margin-bottom:var(--spacing-sm)">
        <h4 style="margin:0 0 8px 0;color:var(--color-primary);font-size:.9rem">1. 콘텐츠 비율 (합계 자유, 자동 정규화)</h4>
        ${sliders}
      </div>

      <div class="settings-section" style="margin-bottom:var(--spacing-sm)">
        <h4 style="margin:0 0 8px 0;color:var(--color-primary);font-size:.9rem">2. 운영 축</h4>
        ${axisRow('민감도', 'sensitivity', [
          { id:'low', label:'low — 일반 문서' },
          { id:'medium', label:'medium' },
          { id:'high', label:'high — 비밀번호/키 포함' },
          { id:'regulated', label:'regulated — GDPR/HIPAA 등' },
        ])}
        ${axisRow('볼륨', 'volume', [
          { id:'light', label:'light (<50/주)' },
          { id:'moderate', label:'moderate (50~500/주)' },
          { id:'heavy', label:'heavy (500+/주)' },
        ])}
        ${axisRow('검색 의도', 'search_intent', [
          { id:'precision', label:'precision — 정확 매칭' },
          { id:'exploration', label:'exploration — 탐색·연관' },
          { id:'temporal', label:'temporal — 최근 우선' },
        ])}
        ${axisRow('협업', 'collaboration', [
          { id:'solo', label:'solo — 혼자' },
          { id:'small_team', label:'small_team (2~5명)' },
          { id:'team', label:'team (5+명)' },
        ])}
      </div>

      <div style="text-align:right;margin-top:var(--spacing-sm)">
        <button class="btn btn-primary" data-action="setup-profile-review">📋 추천 받기</button>
      </div>

      <div id="setup-result" style="margin-top:var(--spacing-md)"></div>
    `;
    Modal.open('📋 다축 프로파일 기반 추천', html, { wide: true });
  },

  // Phase 79: 프리셋 적용 — 5축 폼에 일괄 값 주입
  _applySetupPreset(presetId) {
    const root = this._setupModalRoot();
    if (!root) return;
    const PRESETS = {
      general: {
        content: { general: 100 },
        sensitivity: 'low', volume: 'moderate', search_intent: 'precision', collaboration: 'solo',
      },
      expert: {
        content: { research: 80, code: 20 },
        sensitivity: 'low', volume: 'moderate', search_intent: 'precision', collaboration: 'solo',
      },
      meeting: {
        content: { meeting: 80, general: 20 },
        sensitivity: 'low', volume: 'moderate', search_intent: 'temporal', collaboration: 'small_team',
      },
      code: {
        content: { code: 90, general: 10 },
        sensitivity: 'high', volume: 'moderate', search_intent: 'precision', collaboration: 'solo',
      },
      secure: {
        content: { general: 100 },
        sensitivity: 'high', volume: 'moderate', search_intent: 'precision', collaboration: 'solo',
      },
      team: {
        content: { meeting: 50, general: 50 },
        sensitivity: 'medium', volume: 'heavy', search_intent: 'exploration', collaboration: 'team',
      },
      allround: {
        // 코드 + 회의록 + 연구 + 일반 (할 일 목록은 코드/회의에 흡수)
        content: { code: 25, meeting: 25, research: 25, general: 25 },
        sensitivity: 'medium', volume: 'moderate', search_intent: 'exploration', collaboration: 'solo',
      },
    };
    const p = PRESETS[presetId];
    if (!p) return;
    // 슬라이더 채움
    root.querySelectorAll('input[type=range][data-axis="content"]').forEach(s => {
      const kind = s.dataset.kind;
      const v = p.content[kind] != null ? p.content[kind] : 0;
      s.value = String(v);
      const span = s.nextElementSibling;
      if (span) span.textContent = `${v}%`;
    });
    // select 채움
    ['sensitivity', 'volume', 'search_intent', 'collaboration'].forEach(axis => {
      const sel = root.querySelector(`select[data-axis="${axis}"]`);
      if (sel && p[axis] != null) sel.value = p[axis];
    });
  },

  // Phase 79: 모달 루트 조회 — Modal._overlay 우선, 폴백으로 .modal-overlay
  _setupModalRoot() {
    return (Modal && Modal._overlay) || document.querySelector('.modal-overlay');
  },

  _readSetupProfileFromForm() {
    const root = this._setupModalRoot();
    if (!root) return null;
    // content sliders → content_mix (합 정규화)
    const sliders = root.querySelectorAll('input[type=range][data-axis="content"]');
    const raw = [];
    sliders.forEach(s => {
      const v = parseFloat(s.value || '0');
      if (v > 0) raw.push([s.dataset.kind, v]);
    });
    let content_mix;
    if (raw.length === 0) {
      content_mix = [['general', 1.0]];
    } else {
      const total = raw.reduce((a, [, v]) => a + v, 0);
      content_mix = raw.map(([k, v]) => [k, +(v / total).toFixed(4)]);
    }
    const get = (axis) => {
      const sel = root.querySelector(`select[data-axis="${axis}"]`);
      return sel ? sel.value : null;
    };
    return {
      description: null,
      content_mix,
      sensitivity: get('sensitivity') || 'low',
      volume: get('volume') || 'moderate',
      search_intent: get('search_intent') || 'precision',
      collaboration: get('collaboration') || 'solo',
      user_role: null,
    };
  },

  async _runSetupProfileReview() {
    const profile = this._readSetupProfileFromForm();
    if (!profile) return;
    const result = document.getElementById('setup-result');
    if (result) result.innerHTML = '<p style="color:var(--color-text-muted)">분석 중...</p>';
    try {
      const advice = await API.setupReviewProfile(profile);
      this._renderSetupAdvice(advice, null, profile);
    } catch(e) {
      if (result) result.innerHTML = `<p style="color:var(--color-error)">오류: ${e}</p>`;
    }
  },

  async _runSetupReview(scenarioOverride) {
    // Phase 76: 자유 입력 제거. 시나리오 카드 클릭만으로 진입.
    const scenario = (scenarioOverride || '').trim();
    if (!scenario) {
      const result = document.getElementById('setup-result');
      if (result) result.innerHTML = '<p style="color:var(--color-warning)">시나리오 카드를 선택하세요.</p>';
      return;
    }
    const result = document.getElementById('setup-result');
    if (result) result.innerHTML = '<p style="color:var(--color-text-muted)">분석 중...</p>';
    try {
      const advice = await API.setupReview(scenario, null);
      this._renderSetupAdvice(advice, scenario);
    } catch(e) {
      if (result) result.innerHTML = `<p style="color:var(--color-error)">오류: ${e}</p>`;
    }
  },

  _renderSetupAdvice(advice, scenario, profile) {
    const result = document.getElementById('setup-result');
    if (!result) return;
    const changes = advice.changes || [];
    // profile을 DOM에 보존 (적용 시 사용)
    this._setupActiveProfile = profile || null;
    this._setupActiveScenario = scenario || null;

    const profileSummary = advice.profile ? this._renderProfileSummary(advice.profile) : '';

    if (changes.length === 0) {
      result.innerHTML = `
        <div class="settings-section" style="margin-top:var(--spacing-md)">
          <h4 style="color:var(--color-primary);margin-bottom:var(--spacing-sm)">프로파일</h4>
          ${profileSummary}
          <p style="margin-top:var(--spacing-sm)">${this._escHtml(advice.summary)}</p>
          <p style="color:var(--color-text-dim);margin-top:var(--spacing-sm)">현재 설정이 적합합니다. 추가 변경 권고 없음.</p>
        </div>`;
      return;
    }

    const priorityBadge = (p) => {
      const map = { P0: ['#f87171', 'P0 필수'], P1: ['#fbbf24', 'P1 권장'], P2: ['#94a3b8', 'P2 선택'] };
      const [c, t] = map[p] || ['#94a3b8', p || '-'];
      return `<span style="background:${c};color:#0f172a;font-size:.65rem;padding:2px 6px;border-radius:3px;font-weight:600">${t}</span>`;
    };
    const riskBadge = (r) => {
      const map = {
        low: ['#22c55e', 'low'],
        medium: ['#fbbf24', 'medium'],
        high: ['#fb923c', 'high'],
        critical: ['#ef4444', 'CRITICAL'],
      };
      const [c, t] = map[r] || ['#94a3b8', r || '-'];
      return `<span style="background:${c};color:#0f172a;font-size:.65rem;padding:2px 6px;border-radius:3px;font-weight:600;margin-left:4px">${t}</span>`;
    };
    const evidenceTag = (e) => {
      const map = { heuristic:'경험', benchmark:'측정', literature:'문헌', user_feedback:'피드백' };
      return `<span style="color:var(--color-text-dim);font-size:.7rem;margin-left:4px">[${map[e] || e || '-'}]</span>`;
    };

    let rows = '';
    let hasCritical = false;
    changes.forEach((c) => {
      const restart = c.needs_restart ? '<span style="color:var(--color-warning);font-size:.7rem;margin-left:4px">⚠ restart</span>' : '';
      const isCritical = c.risk === 'critical';
      if (isCritical) hasCritical = true;
      const conflict = c.conflict_note
        ? `<div style="color:var(--color-warning);font-size:.7rem;margin-top:2px">⚠ ${this._escHtml(c.conflict_note)}</div>`
        : '';
      rows += `
        <tr>
          <td style="padding:8px;vertical-align:top"><input type="checkbox" data-setup-path="${this._escHtml(c.path)}" data-risk="${c.risk}" ${isCritical ? '' : 'checked'}></td>
          <td style="padding:8px;vertical-align:top">
            <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap">
              ${priorityBadge(c.priority)}
              ${riskBadge(c.risk)}
              <code style="color:var(--color-primary)">${this._escHtml(c.path)}</code>
              ${restart}${evidenceTag(c.evidence)}
            </div>
            <div style="color:var(--color-text-dim);font-size:.72rem;margin-top:4px">${this._escHtml(c.reason)}</div>
            ${conflict}
          </td>
          <td style="padding:8px;vertical-align:top;color:var(--color-text-dim);max-width:160px;word-break:break-all">${this._escHtml(JSON.stringify(c.current))}</td>
          <td style="padding:8px;vertical-align:top;color:var(--color-success);max-width:160px;word-break:break-all">${this._escHtml(JSON.stringify(c.recommended))}</td>
        </tr>`;
    });

    const criticalWarn = hasCritical ? `
      <div style="background:rgba(239,68,68,.1);border:1px solid #ef4444;padding:8px;border-radius:4px;margin-top:var(--spacing-sm);font-size:.78rem">
        <strong style="color:#ef4444">⚠ Critical 항목 포함</strong> — 데이터 손실 위험 (예: retention 활성화 시 자동 삭제 시작).
        Critical 항목을 적용하려면 <code>아래 체크박스 + Critical 적용 동의</code>가 둘 다 필요합니다.
      </div>` : '';

    const impactBlock = this._renderPipelineImpact(changes);

    result.innerHTML = `
      <div class="settings-section" style="margin-top:var(--spacing-md)">
        <h4 style="color:var(--color-primary);margin-bottom:var(--spacing-sm)">프로파일</h4>
        ${profileSummary}
        <p style="margin:var(--spacing-sm) 0">${this._escHtml(advice.summary)}</p>
        ${impactBlock}
        <table style="width:100%;border-collapse:collapse;font-size:.82rem;table-layout:fixed">
          <thead>
            <tr style="border-bottom:1px solid var(--color-border)">
              <th style="text-align:left;padding:8px;width:40px">적용</th>
              <th style="text-align:left;padding:8px">설정 / 이유</th>
              <th style="text-align:left;padding:8px;width:160px">현재</th>
              <th style="text-align:left;padding:8px;width:160px">추천</th>
            </tr>
          </thead>
          <tbody>${rows}</tbody>
        </table>
        ${criticalWarn}
        <div style="display:flex;justify-content:space-between;align-items:center;margin-top:var(--spacing-md);padding-top:var(--spacing-sm);border-top:1px dashed var(--color-border)">
          <label style="font-size:.78rem;color:var(--color-text-muted);${hasCritical ? '' : 'visibility:hidden'}">
            <input type="checkbox" id="setup-apply-critical"> Critical 항목 적용 동의
          </label>
          <div>
            <span id="setup-apply-status" style="font-size:.72rem;color:var(--color-text-dim);margin-right:var(--spacing-sm)"></span>
            <button class="btn btn-primary" data-action="setup-apply-submit">선택 항목 적용</button>
          </div>
        </div>
      </div>`;
  },

  // Phase 79: ConfigChange.path → 영향 받는 가공/검색 파이프라인 노드 매핑
  _renderPipelineImpact(changes) {
    if (!changes || changes.length === 0) return '';

    // 가공 파이프라인 — PB_NODES의 configSections / configFields 역인덱스
    const processNodes = {}; // path → [nodeLabel, ...]
    Object.entries(this.PB_NODES || {}).forEach(([nid, def]) => {
      (def.configSections || []).forEach(sk => {
        // 섹션 매칭: 변경 path가 sk.* 또는 sk와 정확히 같으면
        changes.forEach(c => {
          if (c.path === sk || c.path.startsWith(sk + '.')) {
            (processNodes[c.path] = processNodes[c.path] || []).push(`${def.icon} ${def.label}`);
          }
        });
      });
      (def.configFields || []).forEach(([sk, fk]) => {
        const full = `${sk}.${fk}`;
        changes.forEach(c => {
          if (c.path === full) {
            (processNodes[c.path] = processNodes[c.path] || []).push(`${def.icon} ${def.label}`);
          }
        });
      });
    });

    // 검색 파이프라인 매핑 — _searchNodeHasSettings의 역방향
    const searchMap = {
      query_sensitive: { sk: 'sensitive', label: '🔒 민감 차단' },
      query_expand:    { sk: 'search',    label: '➕ 쿼리 확장' },
      hybrid:          { sk: 'search',    label: '🔀 Hybrid (Dense+Sparse RRF)' },
      fuse:            { sk: 'search',    label: '🪢 결과 융합' },
      rerank:          { sk: 'rerank',    label: '🎯 리랭킹' },
      win:             { sk: 'search',    label: '🪟 Sentence Window' },
      mmr:             { sk: 'search',    label: '🎲 MMR 다양성' },
      paging:          { sk: 'vector_db', label: '📑 페이징 (top_k)' },
    };
    const searchNodes = {};
    changes.forEach(c => {
      Object.entries(searchMap).forEach(([_, info]) => {
        if (c.path === info.sk || c.path.startsWith(info.sk + '.')) {
          (searchNodes[c.path] = searchNodes[c.path] || []).push(info.label);
        }
      });
    });

    // 노드별 변경 수 집계
    const aggregate = (map) => {
      const nodeCount = {};
      Object.entries(map).forEach(([path, labels]) => {
        labels.forEach(l => { nodeCount[l] = (nodeCount[l] || 0) + 1; });
      });
      return Object.entries(nodeCount)
        .sort((a, b) => b[1] - a[1])
        .map(([label, n]) => `<span style="background:var(--color-surface);padding:3px 8px;border-radius:3px;margin-right:4px;font-size:.74rem;display:inline-block;margin-bottom:3px">${label} <span style="color:var(--color-primary);font-weight:600">${n}</span></span>`)
        .join('');
    };

    const procHtml = aggregate(processNodes);
    const searchHtml = aggregate(searchNodes);
    if (!procHtml && !searchHtml) return '';

    return `
      <div style="margin:var(--spacing-sm) 0;padding:var(--spacing-sm);background:rgba(56,189,248,0.05);border:1px dashed var(--color-border);border-radius:4px">
        <div style="font-size:.78rem;color:var(--color-text-muted);margin-bottom:6px">🔧 영향 받는 파이프라인 노드 (변경 수)</div>
        ${procHtml ? `<div style="margin-bottom:6px"><span style="color:var(--color-text-dim);font-size:.74rem">📥 가공:</span> ${procHtml}</div>` : ''}
        ${searchHtml ? `<div><span style="color:var(--color-text-dim);font-size:.74rem">🔍 검색:</span> ${searchHtml}</div>` : ''}
      </div>`;
  },

  _renderProfileSummary(p) {
    if (!p) return '';
    const mix = (p.content_mix || [])
      .map(([k, v]) => `${k} ${Math.round(v * 100)}%`)
      .join(', ');
    const tag = (l, v) => v ? `<span style="background:var(--color-surface-hover);padding:2px 8px;border-radius:3px;margin-right:4px;font-size:.72rem">${l}: ${v}</span>` : '';
    return `<div style="font-size:.78rem;color:var(--color-text-muted)">
      <div style="margin-bottom:4px"><strong>콘텐츠</strong>: ${mix || 'general'}</div>
      <div>${tag('민감도', p.sensitivity)}${tag('볼륨', p.volume)}${tag('검색', p.search_intent)}${tag('협업', p.collaboration)}</div>
    </div>`;
  },

  async _runSetupApply(scenario) {
    const status = document.getElementById('setup-apply-status');
    const checks = document.querySelectorAll('#setup-result input[type="checkbox"][data-setup-path]');
    const accepted = Array.from(checks).filter(c => c.checked).map(c => c.dataset.setupPath);
    if (accepted.length === 0) {
      if (status) status.textContent = '적용할 항목을 선택하세요.';
      return;
    }
    const criticalCheck = document.getElementById('setup-apply-critical');
    const applyCritical = !!(criticalCheck && criticalCheck.checked);
    // 만약 critical 체크된 path가 선택됐는데 동의 미체크면 경고
    const criticalSelected = Array.from(checks)
      .filter(c => c.checked && c.dataset.risk === 'critical')
      .map(c => c.dataset.setupPath);
    if (criticalSelected.length > 0 && !applyCritical) {
      if (status) {
        status.textContent = `Critical 항목 ${criticalSelected.length}건 적용에 동의 필요`;
        status.style.color = 'var(--color-warning)';
      }
      return;
    }

    if (status) { status.textContent = '적용 중...'; status.style.color = 'var(--color-text-dim)'; }
    try {
      const profile = this._setupActiveProfile;
      const sc = scenario || this._setupActiveScenario;
      const res = profile
        ? await API.setupApplyProfile(profile, accepted, applyCritical)
        : await API.setupApply(sc, accepted);
      if (status) {
        const restart = res.needs_restart ? ' (재시작 필요)' : '';
        status.textContent = `✓ ${(res.applied || []).length}건 적용됨${restart}. 백업: ${res.backup}`;
        status.style.color = 'var(--color-success)';
      }
      this.state.config = null;
      this.state.configMeta = null;
      if (this.state.activeTab === 'pipeline') this.loadPipelineBuilder();
      if (this.state.activeTab === 'settings') this.loadSettings();
    } catch(e) {
      if (status) { status.textContent = '실패: ' + e; status.style.color = 'var(--color-error)'; }
    }
  },

  // ── Actions ──
  // ── Processing ──
  async loadProcessing() {
    const [queueData, progressData, errorsData, crStats] = await Promise.all([
        API.queue(), API.progress(), API.errors(), API.crossrefStats(),
    ]);
    const items = queueData.items || [];
    this.state._progressEvents = progressData.events || [];
    this.state._errorEntries = errorsData.entries || errorsData.errors || [];
    this.state._procItems = items;
    this.state._procPage = this.state._procPage || 1;
    if (!this.state._procListenersAttached) {
      this.state._procSearch = '';
      this.state._procFilter = 'all';
    }

    // 문서 처리 현황 카드
    const s = queueData.stats || queueData;
    const procEl = document.getElementById('q-processing');
    const doneEl = document.getElementById('q-done');
    const failEl = document.getElementById('q-failed');
    const pendEl = document.getElementById('q-pending');
    if (procEl) procEl.textContent = s.processing || 0;
    if (doneEl) doneEl.textContent = s.done || 0;
    if (failEl) failEl.textContent = s.failed || 0;
    if (pendEl) pendEl.textContent = s.pending || 0;

    // 교차 참조 현황 카드
    if (crStats) {
        const totalEl = document.getElementById('cr-total');
        const docsEl = document.getElementById('cr-docs');
        const densEl = document.getElementById('cr-density');
        if (totalEl) totalEl.textContent = crStats.total_relations || 0;
        if (docsEl) docsEl.textContent = crStats.total_documents || 0;
        if (densEl) densEl.textContent = crStats.total_documents > 0 ? ((crStats.total_relations||0)/(crStats.total_documents||1)).toFixed(1) : '-';

        const byTypeEl = document.getElementById('cr-by-type');
        if (byTypeEl && crStats.by_type) {
            const relIcons = { references: '\uD83D\uDCCE', updates: '\uD83D\uDD04', related_topic: '\uD83D\uDD17', supersedes: '\u2B06\uFE0F' };
            byTypeEl.innerHTML = Object.entries(crStats.by_type).map(([type, count]) =>
                `<span style="margin-right:var(--spacing-sm)">${relIcons[type]||'\u2022'} ${type}: ${count}</span>`
            ).join('');
        }
    }

    this._renderProcTable();

    // 검색/필터 이벤트 (최초 1회만 바인딩)
    if (!this.state._procListenersAttached) {
      const searchEl = document.getElementById('proc-search');
      const filterEl = document.getElementById('proc-filter');
      if (searchEl) searchEl.addEventListener('input', () => { this.state._procSearch = searchEl.value; this.state._procPage = 1; this._renderProcTable(); });
      if (filterEl) filterEl.addEventListener('change', () => { this.state._procFilter = filterEl.value; this.state._procPage = 1; this._renderProcTable(); });
      this.state._procListenersAttached = true;
    }
  },

  _renderProcTable() {
    const el = document.getElementById('processing-list');
    if (!el) return;
    const items = this.state._procItems || [];
    const search = (this.state._procSearch || '').toLowerCase();
    const filter = this.state._procFilter || 'all';
    const perPage = 10;
    const page = this.state._procPage || 1;

    // 상태 매핑 (한국어 → 필터값)
    const statusToFilter = { '처리중': 'processing', '완료': 'done', '대기': 'pending' };
    const getFilterKey = (status) => {
      if (!status) return '';
      if (status.startsWith('실패')) return 'failed';
      return statusToFilter[status] || status.toLowerCase();
    };

    // 필터링
    let filtered = items.filter(item => {
        const name = (item.name || item.path || '').toLowerCase();
        if (search && !name.includes(search) && !(item.status||'').toLowerCase().includes(search)) return false;
        if (filter !== 'all' && getFilterKey(item.status) !== filter) return false;
        return true;
    });

    // 정렬: created_at 오름차순 (input 시각 순서 고정). status는 색상/뱃지로만 구분 — 행 점프 방지.
    filtered.sort((a, b) => {
      const ta = a.created_at || a.updated_at || '';
      const tb = b.created_at || b.updated_at || '';
      return ta.localeCompare(tb);
    });

    const totalPages = Math.max(1, Math.ceil(filtered.length / perPage));
    const paged = filtered.slice((page - 1) * perPage, page * perPage);

    if (!paged.length) {
        el.innerHTML = '<p style="color:var(--color-text-muted);font-size:.82rem">작업 내역이 없습니다.</p>';
    } else {
        let html = '<table style="width:100%;font-size:.8rem;border-collapse:collapse"><thead><tr style="border-bottom:1px solid var(--color-border);text-align:left"><th style="padding:6px">파일명</th><th style="padding:6px">상태</th><th style="padding:6px">크기</th><th style="padding:6px">갱신</th></tr></thead><tbody>';
        paged.forEach(item => {
            const name = item.name || (item.path ? item.path.split(/[/\\]/).pop() : '?');
            const status = item.status || '?';
            const statusColor = status === '완료' ? 'var(--color-success)' : status.startsWith('실패') ? 'var(--color-error)' : status === '처리중' ? 'var(--color-primary)' : 'var(--color-text-muted)';
            const sizeKb = item.size_kb != null ? item.size_kb + 'KB' : (item.size_bytes ? (item.size_bytes > 1024 ? (item.size_bytes/1024).toFixed(1)+'KB' : item.size_bytes+'B') : '-');
            const updated = item.updated_at ? item.updated_at.substring(0, 19).replace('T', ' ') : '-';
            const escapedName = this._escHtml(name);
            html += `<tr style="border-bottom:1px solid var(--color-border);cursor:pointer" data-action="show-processing-log" data-name="${escapedName}">
                <td style="padding:6px;font-family:var(--font-mono);max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" title="${escapedName}">${escapedName}</td>
                <td style="padding:6px;color:${statusColor};font-weight:600">${status}${item.is_large ? ' (대용량)' : ''}</td>
                <td style="padding:6px">${sizeKb}</td>
                <td style="padding:6px;color:var(--color-text-dim);font-size:.75rem">${updated}</td>
            </tr>`;
        });
        html += '</tbody></table>';
        el.innerHTML = html;
    }

    // 페이징
    const pagEl = document.getElementById('processing-pagination');
    if (pagEl && totalPages > 1) {
        let pagHtml = '';
        for (let p = 1; p <= totalPages; p++) {
            const active = p === page ? 'font-weight:700;color:var(--color-primary)' : 'color:var(--color-text-muted)';
            pagHtml += `<button data-action="proc-page" data-page="${p}" style="font-size:.75rem;cursor:pointer;background:none;border:none;${active}">${p}</button>`;
        }
        pagEl.innerHTML = pagHtml;
    } else if (pagEl) {
        pagEl.innerHTML = '';
    }
  },

  async showProcessingLog(fileName) {
    const panel = document.getElementById('processing-log-panel');
    const title = document.getElementById('processing-log-title');
    const content = document.getElementById('processing-log-content');
    if (!panel || !content) return;

    // 패널 즉시 열고 로딩 표시 (backend 파일 스캔에 시간 들 수 있음)
    title.textContent = fileName;
    content.textContent = '로그 불러오는 중...';
    panel.style.display = 'block';

    // progress 이벤트에서 해당 파일 로그 필터링
    const events = (this.state._progressEvents || []).filter(e => {
      try {
        const obj = typeof e === 'string' ? JSON.parse(e) : e;
        return obj.file === fileName;
      } catch { return String(e).includes(fileName); }
    });

    // error 로그에서 해당 파일 필터링
    const errors = (this.state._errorEntries || []).filter(e =>
      (e.file || e.filename || '').includes(fileName)
    );

    // 이벤트 → 사람이 읽기 좋은 라벨 매핑 (백엔드 emit_progress 형식 기반)
    const eventLabel = { start: '🟢 시작', step: '⚙️ 단계', done: '✅ 완료', error: '❌ 오류', fragment: '🧩 단편 스킵' };
    const stageLabel = { preprocess: '전처리', classify: 'LLM 분류·가공', verify: '검증' };

    let log = '';
    if (events.length) {
      log += '=== 처리 이벤트 ===\n';
      events.forEach(e => {
        try {
          const obj = typeof e === 'string' ? JSON.parse(e) : e;
          const label = eventLabel[obj.event] || obj.event || 'event';
          let line = label;
          if (obj.stage) line += ` — ${stageLabel[obj.stage] || obj.stage}`;
          if (obj.types) line += ` (분류: ${obj.types})`;
          if (obj.reason) line += ` — ${obj.reason}`;
          log += line + '\n';
        } catch { log += String(e) + '\n'; }
      });
    }
    if (errors.length) {
      log += '\n=== 오류 ===\n';
      errors.forEach(e => {
        log += `[${e.stage || 'error'}] ${e.message || e.reason || JSON.stringify(e)}\n`;
        if (e.suggestion) log += `  → ${e.suggestion}\n`;
      });
    }

    // work queue 상태/시간 정보 추가
    const queueItem = (this.state._procItems || []).find(it => (it.name || (it.path||'').split(/[/\\]/).pop()) === fileName);
    if (queueItem) {
      log += '\n=== 큐 상태 ===\n';
      log += `상태: ${queueItem.status}\n`;
      if (queueItem.created_at) log += `등록: ${queueItem.created_at.substring(0, 19).replace('T', ' ')}\n`;
      if (queueItem.updated_at) log += `갱신: ${queueItem.updated_at.substring(0, 19).replace('T', ' ')}\n`;
      if (queueItem.is_large) log += '대용량 파일 (>40KB)\n';
    }

    // 파일별 pipeline.log 라인 추가 (LLM 호출, 전처리, 검증 등 상세 trace).
    try {
      const fileLog = await API.fileLog(fileName, 300);
      const lines = fileLog?.lines || [];
      if (lines.length) {
        log += '\n=== Pipeline Log ===\n';
        if (fileLog.truncated) log += `(최근 ${lines.length}건만 표시)\n`;
        // 라인 형식: [2026-05-29 19:39:23] [INFO] {"event":"start","file":"X.md"}
        // 또는: 2026-05-29T07:39:23.123456Z  INFO 메시지...
        for (const line of lines) {
          log += line + '\n';
        }
      }
    } catch (e) {
      log += '\n=== Pipeline Log ===\n(파일 로그 조회 실패: ' + (e?.message || e) + ')\n';
    }

    if (!log.trim()) {
      log = `"${fileName}" 에 대한 로그가 없습니다.\n\n`;
      log += '로그는 파일 처리 중에만 기록됩니다.\n';
      log += '파일이 아직 처리되지 않았거나, 로그가 순환되어 삭제되었을 수 있습니다.';
    }

    content.textContent = log;
  },

  async retryFailed() {
    const result = await API.retryFailed();
    if (result.error) { this._showMsg(result.error, 'error'); return; }
    this._showMsg(`${result.retried || 0}건 재처리 대기열에 추가됨`, 'success');
    this.loadProcessing();
  },

  closeProcessingLog() {
    const panel = document.getElementById('processing-log-panel');
    if (panel) panel.style.display = 'none';
  },

  async doSearch() {
    const q = document.getElementById('search-query').value;
    const docType = document.getElementById('doc-type-filter').value;
    const dateFrom = document.getElementById('date-from').value;
    const dateTo = document.getElementById('date-to').value;

    if (q) {
      // 벡터 검색 모드
      const params = { query: q, top_k: 20 };
      if (docType) params.doc_type = docType;
      if (dateFrom) params.date_from = dateFrom;
      if (dateTo) params.date_to = dateTo;
      const data = await API.search(params);
      this.state.searchResults = data.results || [];
      this.state.documents = this.state.searchResults.map(r => ({
        id: r.id, doc_types: r.doc_types, date: r.date, score: r.score,
        hierarchy: r.hierarchy || [],
        access_count: r.access_count,
        topic: r.topic,
      }));
      this.renderDocList();
      // 검색 결과 KG 그래프
      if (this.state.searchResults.length > 0) {
        const kg = await API.kgNeighbors(this.state.searchResults[0].id);
        this.renderKgGraph(this.state.searchResults[0].id, kg);
      }
    } else {
      // 필터만 적용 — 목록 로드
      this.state.docPage = 1;
      this.loadDocuments(docType);
    }
  },

  async showDoc(id) {
    const doc = await API.document(id);
    if (doc.error) return;
    this.state.currentDoc = doc;
    this.renderDocDetail();
    this.renderKGGraph(id);
  },

  async loadDocuments(docType) {
    const params = { page: this.state.docPage, per_page: 20 };
    if (docType) params.doc_type = docType;
    const data = await API.documents(params);
    this.state.documents = data.documents || [];
    this.state.docTotalPages = data.total_pages || 1;
    this.renderDocList();
    this.renderKGGraph();
  },

  // ── Todos ──
  async loadTodos() {
    this.state.todos = await API.todos();
    this.renderTodos();
  },

  renderTodos() {
    const data = this.state.todos;
    if (!data) return;
    document.getElementById('todo-pending').textContent = data.pending || 0;
    document.getElementById('todo-completed').textContent = data.completed || 0;
    const el = document.getElementById('todo-list');
    let html = '<div style="display:flex;justify-content:flex-end;padding:var(--spacing-sm) var(--spacing-md)"><button class="btn btn-primary" data-action="add-todo" style="font-size:.8rem">+ 할일 추가</button></div>';
    if (!data.items || !data.items.length) {
      html += '<div class="todo-item" style="color:var(--color-text-muted)">할일 없음</div>';
    } else {
      html += data.items.map(item => {
        const cls = item.completed ? 'completed' : '';
        const icon = item.completed ? '✅' : '⬜';
        const due = item.due_date ? `<span style="font-size:.7rem;color:var(--color-warning);margin-left:var(--spacing-sm)">${item.due_date}</span>` : '';
        const cat = item.category ? `<span style="font-size:.68rem;color:var(--color-text-dim);margin-left:var(--spacing-sm)">[${this._escHtml(item.category)}]</span>` : '';
        return `<div class="todo-item">
          <span class="todo-checkbox" data-action="toggle-todo" data-id="${this._escHtml(item.id)}" style="cursor:pointer">${icon}</span>
          <span class="todo-text ${cls}">${this._escHtml(item.text)}</span>${cat}${due}
          <span class="todo-date">${item.date || ''}</span>
        </div>`;
      }).join('');
    }
    el.innerHTML = html;
  },

  async toggleTodo(todoId) {
    await API.completeTodo(todoId);
    this.state.todos = null;
    this.loadTodos();
  },

  openAddTodoModal() {
    const bodyHtml = `
      <div class="form-group">
        <label>할일</label>
        <input type="text" id="todo-title" placeholder="할일 내용을 입력하세요">
      </div>
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:var(--spacing-md)">
        <div class="form-group">
          <label>카테고리</label>
          <input type="text" id="todo-category" value="manual" placeholder="예: manual, inbox">
        </div>
        <div class="form-group">
          <label>기한</label>
          <input type="date" id="todo-due">
        </div>
      </div>`;
    Modal.open('할일 추가', bodyHtml, {
      onSave: async (overlay) => {
        const title = overlay.querySelector('#todo-title')?.value?.trim();
        if (!title) throw new Error('할일 내용을 입력하세요');
        const category = overlay.querySelector('#todo-category')?.value?.trim() || 'manual';
        const due = overlay.querySelector('#todo-due')?.value || null;
        const result = await API.addTodo(title, category, due);
        if (result?.error) throw new Error(result.error);
        this.state.todos = null;
        this.loadTodos();
      }
    });
  },

  // ── Verification Metrics ──
  async loadVerificationMetrics() {
    this.state.verificationMetrics = await API.verificationMetrics();
    this.renderVerificationMetrics();
  },

  renderVerificationMetrics() {
    const el = document.getElementById('verification-results');
    const m = this.state.verificationMetrics;
    // G-4 (b): 빈 객체 {}도 빈 상태로 인식. invoke 미동작(HTTP 모드) 또는 응답 미수신 시 placeholder.
    if (!m || typeof m.total !== 'number') {
      el.innerHTML = '<p style="color:var(--color-text-muted)">검증 메트릭이 아직 없습니다. 가공이 완료되면 표시됩니다.</p>';
      return;
    }
    let html = `<div class="grid" style="margin-bottom:var(--spacing-md)">
      <div class="card"><h3>Total</h3><div class="value">${m.total}</div></div>
      <div class="card"><h3>Pass</h3><div class="value" style="color:var(--color-success)">${m.pass}</div></div>
      <div class="card"><h3>Warning</h3><div class="value" style="color:var(--color-warning)">${m.warning}</div></div>
      <div class="card"><h3>Fail</h3><div class="value" style="color:var(--color-error)">${m.fail}</div></div>
    </div>`;
    if (m.recent && m.recent.length) {
      html += '<div class="panel"><table style="width:100%;font-size:.8rem;border-collapse:collapse">';
      html += '<tr style="color:var(--color-text-muted)"><th>Doc</th><th>Structure</th><th>Compress</th><th>KW Cov</th><th>KW Comp</th><th>ROUGE-L</th><th>Entity</th><th>Result</th></tr>';
      m.recent.forEach(r => {
        const color = r.overall === 'pass' ? 'var(--color-success)' : r.overall === 'fail' ? 'var(--color-error)' : 'var(--color-warning)';
        html += `<tr style="border-bottom:1px solid var(--color-border)">
          <td style="padding:4px">${r.doc_id.substring(0,20)}</td>
          <td>${(r.structure*100).toFixed(0)}%</td><td>${(r.compression*100).toFixed(0)}%</td>
          <td>${(r.keyword_coverage*100).toFixed(0)}%</td><td>${(r.keyword_completeness*100).toFixed(0)}%</td>
          <td>${(r.rouge_l*100).toFixed(0)}%</td><td>${(r.entity*100).toFixed(0)}%</td>
          <td style="color:${color};font-weight:600">${r.overall}</td>
        </tr>`;
      });
      html += '</table></div>';
    }
    el.innerHTML = html;
  },

  // ── Phase 93 GUI 가시화: H1 이상 신호 / H3 MCP 카탈로그 / H5 Notion capability / A2 PII 토글 ──

  // Phase 92 H1: audit_trace 이상 신호 카드 (Verification 탭)
  async loadAnomalyReport() {
    try {
      const report = await API.anomalyReport();
      this.renderAnomalyReport(report);
    } catch (e) {
      // 무시 — invoke 미동작(HTTP 모드) 또는 audit_trace 비어있음
    }
  },

  renderAnomalyReport(report) {
    // Verification 탭 안에 신규 div 동적 삽입
    const container = document.getElementById('verification-results');
    if (!container) return;
    let host = document.getElementById('anomaly-report-card');
    if (!host) {
      host = document.createElement('div');
      host.id = 'anomaly-report-card';
      host.className = 'card';
      host.style.cssText = 'margin-top:var(--spacing-md);padding:var(--spacing-md)';
      container.parentNode.insertBefore(host, container.nextSibling);
    }
    if (!report || !report.has_anomaly) {
      host.innerHTML = `
        <h3 style="margin:0 0 var(--spacing-sm) 0;font-size:.95rem">🩺 자동 이상 감지</h3>
        <p style="color:var(--color-text-muted);font-size:.8rem;margin:0">최근 audit_trace ${report ? report.examined_events : 0}건 분석 — 이상 신호 없음.</p>
        <p style="color:var(--color-text-dim);font-size:.72rem;margin:6px 0 0 0">자동 롤백이 아닌 사용자 검토 권고 시스템.</p>
      `;
      return;
    }
    let html = `
      <h3 style="margin:0 0 var(--spacing-sm) 0;font-size:.95rem;color:var(--color-warning)">⚠ 자동 이상 감지 — ${report.signals.length}건</h3>
      <p style="color:var(--color-text-dim);font-size:.72rem;margin:0 0 var(--spacing-sm) 0">
        최근 ${report.examined_events}건 audit_trace 분석. <strong>자동 롤백 아닌 사용자 검토 권고</strong> (lesson 50 메타 룰 20 적용).
      </p>
    `;
    report.signals.forEach(s => {
      html += `
        <div style="padding:var(--spacing-sm);background:var(--color-surface);border-left:3px solid var(--color-warning);border-radius:4px;margin-bottom:var(--spacing-sm)">
          <div style="font-weight:600;font-size:.85rem">${this._escHtml(s.kind)} — stage: <code>${this._escHtml(s.stage)}</code></div>
          <div style="font-size:.78rem;margin-top:4px">${this._escHtml(s.summary)}</div>
          <div style="font-size:.72rem;color:var(--color-text-dim);margin-top:6px">${this._escHtml(s.recommendation)}</div>
        </div>
      `;
    });
    host.innerHTML = html;
  },

  // Phase 92 H3: MCP 도구 다차원 카탈로그 (Settings 탭)
  async loadMcpCatalog() {
    try {
      const data = await API.mcpToolCatalogFull();
      this.renderMcpCatalog(data);
    } catch (e) { /* invoke 미동작 시 무시 */ }
  },

  renderMcpCatalog(data) {
    const host = document.getElementById('mcp-catalog-result');
    if (!host) return;
    if (!data || !data.tools || !data.tools.length) {
      host.innerHTML = '<p style="color:var(--color-text-muted);font-size:.78rem">카탈로그 비어있음</p>';
      return;
    }
    // 카테고리별 그룹화
    const byCategory = {};
    data.tools.forEach(t => {
      (byCategory[t.category] = byCategory[t.category] || []).push(t);
    });
    let html = `<div style="font-size:.72rem;color:var(--color-text-dim);margin-bottom:var(--spacing-sm)">총 ${data.total}개 도구 (Mirage 3차원 등록 패턴).</div>`;
    html += '<table style="width:100%;font-size:.75rem;border-collapse:collapse">';
    html += '<tr style="color:var(--color-text-muted);border-bottom:1px solid var(--color-border)"><th style="text-align:left;padding:4px">도구</th><th style="text-align:left;padding:4px">카테고리</th><th style="text-align:left;padding:4px">상태 변경</th><th style="text-align:left;padding:4px">비용</th></tr>';
    Object.keys(byCategory).sort().forEach(cat => {
      byCategory[cat].forEach(t => {
        const mutates = t.mutates
          ? '<span style="color:var(--color-warning);font-weight:600">⚠ 변경</span>'
          : '<span style="color:var(--color-text-muted)">읽기</span>';
        const cost = t.cost === 'llm-call' ? '<span style="color:var(--color-primary)">LLM</span>'
          : t.cost === 'heavy-compute' ? '<span style="color:var(--color-warning)">heavy</span>'
          : '<span style="color:var(--color-text-muted)">free</span>';
        html += `<tr style="border-bottom:1px solid var(--color-border)">
          <td style="padding:4px;font-family:var(--font-mono)">${this._escHtml(t.name)}</td>
          <td style="padding:4px"><code>${this._escHtml(t.category)}</code></td>
          <td style="padding:4px">${mutates}</td>
          <td style="padding:4px">${cost}</td>
        </tr>`;
      });
    });
    html += '</table>';
    host.innerHTML = html;
  },

  // Phase 91 A2: 출력 PII mask 토글 (Settings 탭)
  async loadPiiMaskToggle() {
    try {
      const data = await API.piiMaskConfig();
      const toggle = document.getElementById('pii-mask-toggle');
      if (toggle) toggle.checked = !!(data && data.output_pii_mask);
    } catch (e) { /* invoke 미동작 시 무시 */ }
  },

  async togglePiiMask() {
    const toggle = document.getElementById('pii-mask-toggle');
    if (!toggle) return;
    try {
      const cfg = await API.getConfig();
      if (!cfg || !cfg.search) return;
      cfg.search.output_pii_mask = toggle.checked;
      await API.updateConfig(cfg);
    } catch (e) {
      // 토글 원복
      toggle.checked = !toggle.checked;
    }
  },

  // Phase 92 H5: 원격 저장소 capability (Pipeline 인스펙터)
  async loadRemoteStorageCapabilities() {
    try {
      return await API.remoteStorageCapabilities();
    } catch (e) { return null; }
  },

  // Phase 93 H5: remote_upload 인스펙터의 capability 영역을 비동기 채움.
  async _loadRemoteStorageCapInline() {
    const host = document.getElementById('remote-storage-cap');
    if (!host) return;
    const caps = await this.loadRemoteStorageCapabilities();
    if (!caps) {
      host.innerHTML = '<div class="info-line" style="font-size:.75rem;color:var(--color-text-muted)">capability 정보 없음 (어댑터 비활성 또는 invoke 미동작)</div>';
      return;
    }
    const checkMark = v => v ? '✅' : '❌';
    const modeWarn = caps.backend === 'notion' && caps.active_mode === 'attach'
      ? '<div style="color:var(--color-warning);font-size:.72rem;margin-top:4px">⚠ Notion attach 모드는 upload 명시적 미지원. S3/WebDAV 권장 또는 mode=page로 변경.</div>'
      : '';
    host.innerHTML = `
      <div class="info-line" style="font-weight:600">🗂 활성 백엔드: <code>${this._escHtml(caps.backend)}</code></div>
      <div style="font-size:.72rem;color:var(--color-text-dim);margin-top:4px">
        is_configured: ${checkMark(caps.is_configured)} | upload: ${checkMark(caps.can_upload)} | download: ${checkMark(caps.can_download)} | list: ${checkMark(caps.can_list)} | delete: ${checkMark(caps.can_delete)}
      </div>
      <div style="font-size:.72rem;color:var(--color-text-dim);margin-top:2px">
        hard_delete: ${checkMark(caps.supports_hard_delete)} ${caps.supports_hard_delete ? '' : '(archive/soft delete만)'}
      </div>
      ${caps.mode_options && caps.mode_options.length
        ? `<div style="font-size:.72rem;color:var(--color-text-dim);margin-top:2px">mode: <code>${this._escHtml(caps.active_mode || '-')}</code> / options: ${caps.mode_options.map(m => `<code>${m}</code>`).join(', ')}</div>`
        : ''}
      ${modeWarn}
    `;
  },

  // ── Topics ──
  async loadTopics() {
    const data = await API.topics();
    this.state.topics = data.topics || [];
    this.renderTopics();
  },

  renderTopics() {
    const el = document.getElementById('topics-list');
    const topics = this.state.topics;
    if (!topics.length) { el.innerHTML = '<p style="color:var(--color-text-muted)">토픽 없음</p>'; return; }

    const searchVal = (this.state._topicSearch || '').toLowerCase();
    const sortBy = this.state._topicSort || 'name';

    let filtered = topics.filter(t => {
      if (!searchVal) return true;
      return (t.name || '').toLowerCase().includes(searchVal)
        || (t.path || '').toLowerCase().includes(searchVal)
        || (t.doc_type || '').toLowerCase().includes(searchVal);
    });

    if (sortBy === 'date') {
      filtered.sort((a, b) => (b.modified || 0) - (a.modified || 0));
    } else if (sortBy === 'type') {
      filtered.sort((a, b) => (a.doc_type || '').localeCompare(b.doc_type || ''));
    } else {
      filtered.sort((a, b) => (a.name || '').localeCompare(b.name || ''));
    }

    let html = `<div style="display:flex;gap:var(--spacing-sm);margin-bottom:var(--spacing-md);align-items:center">
      <input type="text" id="topic-search" placeholder="토픽 검색..." value="${this._escHtml(this.state._topicSearch || '')}" style="flex:1;font-size:.82rem;padding:6px 10px;background:var(--color-bg);border:1px solid var(--color-border);color:var(--color-text);border-radius:4px">
      <select id="topic-sort" style="font-size:.82rem;padding:6px;background:var(--color-bg);border:1px solid var(--color-border);color:var(--color-text);border-radius:4px">
        <option value="name"${sortBy==='name'?' selected':''}>이름순</option>
        <option value="date"${sortBy==='date'?' selected':''}>날짜순</option>
        <option value="type"${sortBy==='type'?' selected':''}>유형순</option>
      </select>
    </div>`;

    const grouped = {};
    filtered.forEach(t => { (grouped[t.doc_type || '기타'] = grouped[t.doc_type || '기타'] || []).push(t); });
    for (const [type, items] of Object.entries(grouped)) {
      html += `<h3 class="section-title">${type} (${items.length})</h3>`;
      html += items.map(t =>
        `<div class="topic-card" data-action="open-topic" data-path="${t.path}">
          <div class="topic-type">${t.doc_type || '기타'}</div>
          <div style="font-weight:600;margin:4px 0">${t.name}</div>
          <div style="font-size:.72rem;color:var(--color-text-dim)">${t.path}</div>
          <div class="doc-id">${(t.size / 1024).toFixed(1)} KB</div>
        </div>`
      ).join('');
    }
    el.innerHTML = html;

    // 검색/정렬 이벤트 바인딩
    const searchInput = document.getElementById('topic-search');
    const sortSelect = document.getElementById('topic-sort');
    if (searchInput) searchInput.addEventListener('input', () => { vm.state._topicSearch = searchInput.value; vm.renderTopics(); });
    if (sortSelect) sortSelect.addEventListener('change', () => { vm.state._topicSort = sortSelect.value; vm.renderTopics(); });
  },

  async openTopic(path) {
    const data = await API.topic(path);
    if (data.error) return;
    this.state.currentTopicPath = path;

    const bodyHtml = `
      <div style="display:flex;gap:var(--spacing-sm);margin-bottom:var(--spacing-sm)">
        <button class="btn btn-secondary" style="font-size:.75rem" id="topic-modal-revert">되돌리기</button>
        <button class="btn btn-secondary" style="font-size:.75rem;color:var(--color-error)" id="topic-modal-delete">삭제</button>
      </div>
      <textarea id="topic-modal-content" style="width:100%;min-height:400px;font-family:var(--font-mono);font-size:.82rem;resize:vertical">${this._escHtml(data.content)}</textarea>`;

    const overlay = Modal.open(`토픽 편집 — ${path}`, bodyHtml, {
      wide: true,
      saveLabel: '저장',
      onSave: async (ov) => {
        const content = ov.querySelector('#topic-modal-content')?.value;
        const result = await API.updateTopic(path, content);
        if (result.error) throw new Error(result.error);
        this.loadTopics();
      }
    });

    // 되돌리기
    overlay.querySelector('#topic-modal-revert')?.addEventListener('click', async () => {
      const fresh = await API.topic(path);
      const ta = overlay.querySelector('#topic-modal-content');
      if (ta && !fresh.error) ta.value = fresh.content;
    });
    // 삭제
    overlay.querySelector('#topic-modal-delete')?.addEventListener('click', async () => {
      if (!confirm(`"${path}" 토픽을 삭제하시겠습니까?`)) return;
      const result = await API.updateTopic(path, '');
      if (result.ok || !result.error) {
        Modal.close();
        this._showMsg('토픽이 삭제되었습니다', 'success');
        this.loadTopics();
      } else {
        Modal.setStatus(`삭제 실패: ${result.error}`, 'error');
      }
    });
  },


  _editingCredId: null,

  _credentialDynamicFields: {
    claude_cli: `
      <div class="form-group"><label>프로필 경로</label>
      <div class="field-help">Claude CLI 설정 디렉토리 (CLAUDE_CONFIG_DIR). 비워두면 기본 프로필 사용.</div>
      <input type="text" id="cred-profile-path" placeholder="예: C:\\Users\\me\\.claude-work"></div>`,
    ollama: `
      <div class="form-group"><label>서버 URL</label>
      <div class="field-help">Ollama 서버 주소</div>
      <input type="text" id="cred-url" value="http://localhost:11434" placeholder="http://localhost:11434"></div>
      <div class="form-group"><label>모델</label>
      <div class="field-help">사용할 모델명</div>
      <input type="text" id="cred-model" value="llama3" placeholder="llama3"></div>`,
    anthropic_api: `
      <div class="form-group"><label>API 키</label>
      <div class="field-help">Anthropic API 키 (sk-ant-...)</div>
      <input type="password" id="cred-api-key" placeholder="sk-ant-..."></div>`,
    openai_api: `
      <div class="form-group"><label>API 키</label>
      <div class="field-help">OpenAI API 키 (sk-...)</div>
      <input type="password" id="cred-api-key" placeholder="sk-..."></div>
      <div class="form-group"><label>모델</label>
      <div class="field-help">사용할 모델명</div>
      <input type="text" id="cred-model" value="gpt-4o" placeholder="gpt-4o"></div>`,
    gemini: `
      <div class="form-group"><label>API 키</label>
      <div class="field-help">Google Gemini API 키</div>
      <input type="password" id="cred-api-key" placeholder="API 키"></div>
      <div class="form-group"><label>모델</label>
      <div class="field-help">사용할 모델명</div>
      <input type="text" id="cred-model" value="gemini-2.0-flash" placeholder="gemini-2.0-flash"></div>`,
  },

  _onboardingStep: 0,
  _onboardingTotal: 4,

  startOnboarding() {
    this._onboardingStep = 1;
    this._showOnboardingStep();
  },

  _showOnboardingStep() {
    const step = this._onboardingStep;
    const total = this._onboardingTotal;
    const isLast = step === total;

    let title = '';
    let body = '';
    let saveLabel = isLast ? '완료' : '다음 →';
    let onSave = null;

    if (step === 1) {
      title = `1/${total} · 환영합니다`;
      body = `
        <p style="font-size:.9rem;line-height:1.6">
          file-pipeline은 inbox 폴더에 넣은 문서를 자동으로 가공하고,
          대시보드에서 검색·분석할 수 있는 도구입니다.
        </p>
        <p style="font-size:.85rem;color:var(--color-text-dim);line-height:1.6">
          총 4단계를 안내합니다.<br>
          ① 환영 → ② 크레덴셜 등록 → ③ inbox 사용법 → ④ 100개 도달 후 최적화
        </p>`;
      onSave = async () => { this._advanceOnboarding(); };
    } else if (step === 2) {
      title = `2/${total} · 크레덴셜 등록`;
      body = `
        <p style="font-size:.85rem;line-height:1.6;margin-bottom:var(--spacing-sm)">
          LLM API 키를 1개 이상 등록하면 문서 가공이 시작됩니다.<br>
          <span style="color:var(--color-text-dim)">
            추천: Claude CLI (로컬 실행) 또는 Anthropic API.
            나머지 프로바이더는 나중에 Settings 탭에서 추가할 수 있습니다.
          </span>
        </p>
        <button class="btn btn-primary" data-action="onboard-open-cred-form" style="margin-bottom:var(--spacing-sm)">
          크레덴셜 등록 폼 열기
        </button>
        <p style="font-size:.78rem;color:var(--color-text-muted)">
          이미 등록했거나 나중에 추가하려면 "다음"을 누르세요.
        </p>`;
      saveLabel = '다음 → (건너뛰기)';
      onSave = async () => { this._advanceOnboarding(); };
    } else if (step === 3) {
      title = `3/${total} · inbox에 파일 넣기`;
      body = `
        <p style="font-size:.9rem;line-height:1.6">
          inbox 폴더에 파일을 복사하면 자동으로 감지·가공됩니다.
        </p>
        <ul style="font-size:.85rem;line-height:1.8;padding-left:1.2em">
          <li>상단 <strong>감지 ON</strong> 버튼이 켜져 있어야 자동 감지가 동작합니다.</li>
          <li>가공 진행 상황은 <strong>Processing</strong> 탭에서 확인할 수 있습니다.</li>
          <li>가공이 끝난 문서는 <strong>Documents</strong> 탭에서 검색 가능합니다.</li>
        </ul>
        <p style="font-size:.78rem;color:var(--color-text-dim);margin-top:var(--spacing-sm)">
          💡 inbox 폴더 경로는 Settings 탭 → input_source 노드에서 변경할 수 있습니다.
        </p>`;
      onSave = async () => { this._advanceOnboarding(); };
    } else if (step === 4) {
      title = `4/${total} · 100개 도달 후 설정 최적화`;
      body = `
        <p style="font-size:.9rem;line-height:1.6">
          문서 100개 이상 가공되면 사용 패턴 기반 자동 추천을 받을 수 있습니다.
        </p>
        <ul style="font-size:.85rem;line-height:1.8;padding-left:1.2em">
          <li>검색 모드 사용 분포 → 권장 모드 추천</li>
          <li>doc_type 분포 → 동작 모듈 추천</li>
          <li>LLM 캐시 히트율 → 비용 절감 조언</li>
        </ul>
        <p style="font-size:.85rem;margin-top:var(--spacing-sm)">
          100개 이상 가공된 후 <strong>Settings 탭 → 자동 추천 카드</strong>에서 결과를 확인하세요.
        </p>
        <p style="font-size:.78rem;color:var(--color-text-muted);margin-top:var(--spacing-sm)">
          이 안내는 상단 🧭 온보드 버튼으로 다시 열 수 있습니다.
        </p>`;
      onSave = async () => { this._onboardingStep = 0; };
    }

    Modal.open(title, body, { onSave, saveLabel });
  },

  _advanceOnboarding() {
    this._onboardingStep += 1;
    setTimeout(() => this._showOnboardingStep(), 50);
  },

  _onboardOpenCredForm() {
    Modal.close();
    this._onboardingResumeAfterCred = true;
    this.showCredentialForm();
  },

  _onboardingAskSetDefault(name) {
    const currentDefault = this.state.config?.llm?.default_credential || null;
    const alreadyDefault = currentDefault === name;
    const body = alreadyDefault
      ? `<p style="font-size:.9rem;line-height:1.6">
           <strong>${this._escHtml(name)}</strong>이(가) 이미 기본 크레덴셜로 설정되어 있습니다.
         </p>`
      : `<p style="font-size:.9rem;line-height:1.6">
           방금 등록한 <strong>${this._escHtml(name)}</strong>을(를) 기본 크레덴셜로 설정하시겠습니까?
         </p>
         <p style="font-size:.78rem;color:var(--color-text-dim);line-height:1.6">
           기본 크레덴셜은 문서 가공 시 자동으로 사용됩니다.
           ${currentDefault ? `현재 기본: <strong>${this._escHtml(currentDefault)}</strong>` : '현재 설정된 기본 크레덴셜이 없습니다.'}
         </p>`;

    Modal.open('기본 크레덴셜 설정', body, {
      onSave: async () => {
        if (!alreadyDefault) await this.setDefaultCredential(name);
        setTimeout(() => this._advanceOnboarding(), 100);
      },
      saveLabel: alreadyDefault ? '다음 →' : '기본으로 설정 + 다음 →'
    });
  },

  showCredentialForm(credData) {
    this._editingCredId = credData?.id || null;
    const isEdit = !!credData;
    const title = isEdit ? '크레덴셜 편집' : '크레덴셜 추가';

    const bodyHtml = `
      <div class="form-group">
        <label>이름</label>
        <input type="text" id="cred-name" placeholder="예: 회사 Claude, 개인 OpenAI" value="${this._escHtml(credData?.name || '')}">
      </div>
      <div class="form-group">
        <label>프로바이더</label>
        <select id="cred-provider">
          <option value="">선택하세요</option>
          <option value="claude_cli"${credData?.provider === 'claude_cli' ? ' selected' : ''}>Claude CLI</option>
          <option value="anthropic_api"${credData?.provider === 'anthropic_api' ? ' selected' : ''}>Anthropic API</option>
          <option value="openai_api"${credData?.provider === 'openai_api' ? ' selected' : ''}>OpenAI API</option>
          <option value="ollama"${credData?.provider === 'ollama' ? ' selected' : ''}>Ollama</option>
          <option value="gemini"${credData?.provider === 'gemini' ? ' selected' : ''}>Gemini</option>
        </select>
      </div>
      <div id="cred-dynamic-fields"></div>`;

    const overlay = Modal.open(title, bodyHtml, {
      onSave: async (ov) => {
        const name = ov.querySelector('#cred-name').value.trim();
        const provider = ov.querySelector('#cred-provider').value;
        if (!name || !provider) throw new Error('이름과 프로바이더를 입력하세요');
        const cred = { name, provider };
        if (this._editingCredId) cred.id = this._editingCredId;
        const apiKey = ov.querySelector('#cred-api-key');
        const url = ov.querySelector('#cred-url');
        const model = ov.querySelector('#cred-model');
        if (apiKey) cred.api_key = apiKey.value;
        if (url) cred.url = url.value;
        if (model) cred.model = model.value;
        const profilePath = ov.querySelector('#cred-profile-path');
        if (profilePath && profilePath.value.trim()) cred.profile_path = profilePath.value.trim();
        const result = await API.saveCredential(cred);
        if (result.error) throw new Error(result.error);
        this._editingCredId = null;
        this._loadInlineCredentials();
        if (this._onboardingResumeAfterCred) {
          this._onboardingResumeAfterCred = false;
          setTimeout(() => this._onboardingAskSetDefault(cred.name), 100);
        }
      }
    });

    // 프로바이더 변경 시 동적 필드 업데이트
    const provSel = overlay.querySelector('#cred-provider');
    provSel.addEventListener('change', () => this._updateModalCredFields(overlay));
    // 초기 동적 필드 렌더
    this._updateModalCredFields(overlay);

    // 편집 시 기존 값 채우기 (동적 필드 렌더 후)
    if (credData) {
      setTimeout(() => {
        const urlEl = overlay.querySelector('#cred-url');
        const modelEl = overlay.querySelector('#cred-model');
        const keyEl = overlay.querySelector('#cred-api-key');
        const profileEl = overlay.querySelector('#cred-profile-path');
        if (urlEl && credData.url) urlEl.value = credData.url;
        if (modelEl && credData.model) modelEl.value = credData.model;
        if (keyEl && credData.has_api_key) keyEl.placeholder = '(변경하려면 입력)';
        if (profileEl && credData.profile_path) profileEl.value = credData.profile_path;
      }, 50);
    }
  },

  async editCredential(credId) {
    const creds = this.state._cachedCredentials || [];
    const cred = creds.find(c => c.id === credId);
    if (!cred) return;
    this.showCredentialForm(cred);
  },

  _updateModalCredFields(overlay) {
    const provider = overlay.querySelector('#cred-provider').value;
    const container = overlay.querySelector('#cred-dynamic-fields');
    container.innerHTML = this._credentialDynamicFields[provider] || '<p style="color:var(--color-text-muted)">프로바이더를 선택하세요</p>';
  },

  async deleteCredentialByName(name) {
    if (!confirm(`"${name}" 크레덴셜을 삭제하시겠습니까?`)) return;
    await API.deleteCredential(name);
    // 삭제된 크레덴셜이 기본이었으면 해제
    if (this.state.config?.llm?.default_credential === name) {
      this.state.config.llm.default_credential = null;
      await API.updateConfig(this.state.config);
    }
    // Pipeline 노드에서 삭제된 credential 참조 제거
    if (this.pb?.nodeValues) {
      const cred = (this.state._cachedCredentials || []).find(c => c.name === name);
      const credId = cred?.id;
      if (credId) {
        for (const node of Object.values(this.pb.nodeValues)) {
          if (node.credential === credId) node.credential = '';
        }
      }
    }
    this._loadInlineCredentials();
  },

  async setDefaultCredential(name) {
    if (!this.state.config) return;
    if (!this.state.config.llm) this.state.config.llm = {};
    this.state.config.llm.default_credential = name;
    // 크레덴셜의 provider도 llm.provider에 반영
    const cred = (this.state._cachedCredentials || []).find(c => c.name === name);
    if (cred) this.state.config.llm.provider = cred.provider;
    const result = await API.updateConfig(this.state.config);
    if (result?.errors) {
      this._showMsg(result.errors.join(', '), 'error');
      return;
    }
    this._showMsg(`"${name}" 을(를) 기본 크레덴셜로 설정했습니다`, 'success');
    this._loadInlineCredentials();
  },


  // ── Settings ──
  // C1: Decision Log 카드 렌더 (Settings 탭 상단). 필터 + 정렬 반영.
  async loadDecisionLog() {
    const host = document.getElementById('decision-log-list');
    const countEl = document.getElementById('dl-filter-count');
    if (!host) return;
    const statusFilter = (document.getElementById('dl-filter-status') || {}).value || 'suggested';
    const sourceFilter = (document.getElementById('dl-filter-source') || {}).value || 'all';
    const sortOrder = (document.getElementById('dl-filter-sort') || {}).value || 'desc';
    try {
      const data = await API.setupDecisionLogList(200);
      let entries = data.entries || [];
      if (statusFilter !== 'all') entries = entries.filter(e => e.decision === statusFilter);
      if (sourceFilter !== 'all') entries = entries.filter(e => e.source === sourceFilter);
      if (sortOrder === 'asc') entries.sort((a, b) => (a.decided_at || '').localeCompare(b.decided_at || ''));
      // desc는 API 기본 (decided_at DESC)

      if (countEl) countEl.textContent = `${entries.length}건`;

      if (entries.length === 0) {
        host.innerHTML = '<p style="color:var(--color-text-dim)">조건에 맞는 항목 없음. 필터를 변경하거나 [분석 실행]으로 새 추천을 생성하세요.</p>';
        return;
      }

      const renderRow = (e) => {
        const idCell = e.id ?? '-';
        const reason = (e.reason || '').replace(/</g, '&lt;');
        const evidence = (e.evidence || '').replace(/</g, '&lt;');
        const after = (e.after_value || '').replace(/</g, '&lt;');
        const source = (e.source || '-').replace(/</g, '&lt;');
        const at = (e.decided_at || '').slice(0, 19).replace('T', ' ');
        let decBadge;
        switch (e.decision) {
          case 'suggested':         decBadge = `<span style="color:var(--color-warning)">⏳ 검토</span>`; break;
          case 'accepted':          decBadge = `<span style="color:var(--color-success)">✓ 적용됨</span>`; break;
          case 'rejected':          decBadge = `<span style="color:var(--color-text-dim)">✕ 거부</span>`; break;
          case 'critical_skipped':  decBadge = `<span style="color:var(--color-error)">⚠ critical 스킵</span>`; break;
          default:                  decBadge = `<span>${(e.decision || '-')}</span>`;
        }
        const canRollback = e.decision === 'accepted' && e.snapshot_id && !e.rolled_back;
        const actions = e.decision === 'suggested'
          ? `<button class="btn btn-sm btn-primary" data-action="accept-suggested" data-id="${idCell}" style="font-size:.7rem;padding:2px 6px">Accept</button>
             <button class="btn btn-sm btn-secondary" data-action="reject-suggested" data-id="${idCell}" style="font-size:.7rem;padding:2px 6px;margin-left:4px">Reject</button>`
          : (canRollback
              ? `<button class="btn btn-sm btn-secondary" data-action="rollback-snapshot" data-snapshot-id="${e.snapshot_id}" style="font-size:.7rem;padding:2px 6px" title="이 변경을 .bak으로 되돌립니다">↶ Rollback</button>`
              : '');
        return `<tr>
          <td>${decBadge}</td>
          <td><code>${e.path}</code></td>
          <td><code>${after}</code></td>
          <td>${reason}<br><span style="color:var(--color-text-dim);font-size:.7rem">${source} · ${at} · ${evidence}</span></td>
          <td style="white-space:nowrap">${actions}</td>
        </tr>`;
      };

      let html = '<table class="doc-table" style="width:100%"><thead><tr>'
        + '<th style="width:9em">상태</th>'
        + '<th>설정 경로</th>'
        + '<th>제안 값</th>'
        + '<th>근거</th>'
        + '<th style="width:9em">조치</th>'
        + '</tr></thead><tbody>';
      html += entries.map(renderRow).join('');
      html += '</tbody></table>';
      host.innerHTML = html;
    } catch (e) {
      host.innerHTML = `<p style="color:var(--color-error)">Decision Log 로드 실패: ${e}</p>`;
    }
  },

  // C1 임계값 카드 렌더 (Settings 탭)
  async loadC1Thresholds() {
    const host = document.getElementById('c1-thresholds-list');
    if (!host) return;
    try {
      const data = await API.c1ThresholdsList();
      const defaults = data.defaults || {};
      const overrides = {};
      (data.overrides || []).forEach(o => { overrides[o.key] = o.value; });

      const rows = [
        { key: 'mode_min_total',      label: '검색 mode 최소 누적' },
        { key: 'mode_dominant_ratio', label: 'mode dominant 비율 (0~1)' },
        { key: 'crag_min_total',      label: 'CRAG 최소 누적' },
        { key: 'crag_incorrect_ratio',label: 'CRAG incorrect 비율 (0~1)' },
        { key: 'processed_min',       label: '처리 최소 누적' },
        { key: 'quarantine_ratio',    label: 'quarantine 비율 (0~1)' },
        { key: 'verify_pass_min',     label: 'verify pass 최소 비율 (0~1)' },
      ];

      let html = '<table class="doc-table" style="width:100%"><thead><tr>'
        + '<th>키</th><th style="width:6em">디폴트</th><th style="width:10em">사용자 값</th><th style="width:5em">조치</th>'
        + '</tr></thead><tbody>';
      rows.forEach(r => {
        const def = defaults[r.key];
        const cur = overrides[r.key];
        const placeholder = def != null ? String(def) : '';
        const value = cur != null ? cur : '';
        html += `<tr>
          <td><div><code>${r.key}</code></div><div style="color:var(--color-text-dim);font-size:.7rem">${r.label}</div></td>
          <td><code>${placeholder}</code></td>
          <td><input data-c1-key="${r.key}" value="${value}" placeholder="${placeholder}" style="width:100%;font-size:.72rem;padding:3px 6px"></td>
          <td><button class="btn btn-sm btn-primary" data-action="c1-save" data-key="${r.key}" style="font-size:.7rem;padding:2px 6px">저장</button></td>
        </tr>`;
      });
      html += '</tbody></table>';
      host.innerHTML = html;
    } catch (e) {
      host.innerHTML = `<p style="color:var(--color-error)">임계값 로드 실패: ${e}</p>`;
    }
  },

  // C2 PII 패턴 카드 렌더 (Settings 탭)
  async loadPiiPatterns() {
    const host = document.getElementById('pii-patterns-list');
    if (!host) return;
    try {
      const data = await API.piiPatternsList();
      const builtin = data.builtin || [];
      const user = data.user_patterns || [];
      const escape = (s) => String(s || '').replace(/</g, '&lt;');
      let html = '<table class="doc-table" style="width:100%"><thead><tr>'
        + '<th style="width:9em">이름</th><th>정규식</th><th style="width:6em">상태</th><th style="width:5em">조치</th>'
        + '</tr></thead><tbody>';
      html += '<tr><td colspan="4" style="background:var(--color-bg);color:var(--color-text-dim);font-size:.72rem;padding:4px 8px">디폴트 (코드 고정, 수정 불가)</td></tr>';
      builtin.forEach(name => {
        html += `<tr><td><code>${escape(name)}</code></td><td colspan="2" style="color:var(--color-text-dim)">코드 패턴</td><td>-</td></tr>`;
      });
      html += '<tr><td colspan="4" style="background:var(--color-bg);color:var(--color-text-dim);font-size:.72rem;padding:4px 8px">사용자 정의 (' + user.length + ')</td></tr>';
      if (user.length === 0) {
        html += '<tr><td colspan="4" style="color:var(--color-text-dim);text-align:center;padding:8px">없음 — 아래에서 추가</td></tr>';
      } else {
        user.forEach(p => {
          const state = p.enabled ? '<span style="color:var(--color-success)">활성</span>' : '<span style="color:var(--color-text-dim)">비활성</span>';
          html += `<tr>
            <td><code>${escape(p.name)}</code></td>
            <td><code style="font-size:.7rem">${escape(p.pattern)}</code></td>
            <td>${state}</td>
            <td><button class="btn btn-sm btn-secondary" data-action="pii-remove" data-name="${escape(p.name)}" style="font-size:.7rem;padding:2px 6px">제거</button></td>
          </tr>`;
        });
      }
      html += '</tbody></table>';
      host.innerHTML = html;
    } catch (e) {
      host.innerHTML = `<p style="color:var(--color-error)">PII 패턴 로드 실패: ${e}</p>`;
    }
  },

  async loadSettings() {
    try {
      const data = await API.getConfig();
      this.state.config = data.config || null;
      this.state.configMeta = data.metadata || null;
    } catch(e) {
      console.error('loadSettings:', e);
    }
    this.renderSettings();
  },

  // Phase 65: 서브탭별 mini-Settings 렌더 (특정 config 섹션만 표시)
  async renderSubtabConfig(hostId, sectionKeys, opts = {}) {
    const host = document.getElementById(hostId);
    if (!host) return;
    if (!this.state.config || !this.state.configMeta) {
      try {
        const data = await API.getConfig();
        this.state.config = data.config || null;
        this.state.configMeta = data.metadata || null;
      } catch(e) { console.error('renderSubtabConfig:', e); }
    }
    if (!this.state.config || !this.state.configMeta) {
      host.innerHTML = '<p style="color:var(--color-text-muted)">설정을 불러올 수 없습니다.</p>';
      return;
    }
    const meta = this.state.configMeta;
    const config = this.state.config;
    const sectionLabels = {
      'preprocessing':'전처리','compression':'압축','sensitive':'민감 문서',
      'vector_db':'벡터 DB','verification':'검증 설정','verification.thresholds':'검증 임계값',
      'logging':'로깅','schedule':'스케줄','paths':'경로 설정','max_workers':'동시성',
      'chunking':'청킹','crossref':'교차참조','rerank':'리랭킹',
      'remote_storage':'원격 저장소','retention':'보존 정책','models':'LLM 모델',
      'llm':'LLM 프로바이더',
    };
    let html = '';
    if (opts.title) html += `<h2 style="color:var(--color-primary);margin-bottom:var(--spacing-md)">${opts.title}</h2>`;
    if (opts.guide) html += `<p class="section-guide" style="margin-bottom:var(--spacing-md)">${opts.guide}</p>`;
    for (const sk of sectionKeys) {
      const fields = meta[sk];
      if (!fields) continue;
      const label = sectionLabels[sk] || sk;
      html += `<div class="settings-section" style="margin-bottom:var(--spacing-lg)"><h3 style="color:var(--color-primary);font-size:.95rem;margin-bottom:var(--spacing-sm)">${label}</h3><div class="section-body">`;
      // paths 섹션에 기본 inbox readonly
      if (sk === 'paths') {
        const basePath = config.paths?.inbox || config.paths?.base || '(exe 디렉토리)/inbox';
        html += `<div class="settings-field" style="padding:4px 0">
          <div class="settings-field-header"><label>inbox (기본)</label></div>
          <div class="field-help">기본 감시 폴더 (자동 설정, readonly)</div>
          <input type="text" value="${this._escHtml(basePath)}" readonly style="opacity:.6;cursor:default">
        </div>`;
      }
      for (const [field, info] of Object.entries(fields)) {
        const value = this._getNestedValue(config, sk, field);
        const input = this._renderField(sk, field, info, value);
        html += `<div class="settings-field" style="padding:4px 0"><div class="settings-field-header"><label>${field}</label></div><div class="field-help">${info.description}</div>${input}</div>`;
      }
      html += '</div></div>';
    }
    // 저장 버튼 (편집 가능 섹션이 있을 때)
    if (sectionKeys.length > 0 && !opts.readonly) {
      html += `<div style="display:flex;gap:var(--spacing-sm);margin-top:var(--spacing-md);padding-top:var(--spacing-md);border-top:1px solid var(--color-border)">
        <button class="btn btn-primary" data-action="save-settings">저장</button>
        <button class="btn btn-secondary" data-action="reset-settings">초기화</button>
      </div>`;
    }
    host.innerHTML = html;
  },

  renderSettings() {
    const form = document.getElementById('settings-form');
    const nav = document.getElementById('settings-nav');
    if (!this.state.config || !this.state.configMeta) {
      nav.innerHTML = '';
      form.innerHTML = invoke
        ? '<p style="color:var(--color-text-muted)">설정을 불러올 수 없습니다. 서비스 연결을 확인하세요.</p>'
        : '<p style="color:var(--color-text-muted)">설정 편집은 Tauri 앱에서만 가능합니다. pipeline.toml을 직접 편집하세요.</p>';
      return;
    }
    const meta = this.state.configMeta;
    const config = this.state.config;

    // Phase 66/70: Settings 4그룹 (Hooks 신규 추가)
    const groups = [
      ['credentials', '크레덴셜 관리', '크레덴셜 등록 · 기본 설정', []],
      ['sys-general', '일반', '로깅', ['logging']],
      ['sys-ops', '운영', '자동 추천 · 임계값 · PII · MCP 카탈로그', []],
      ['sys-hooks', '이벤트 훅', '가공 단계 이벤트에 외부 명령 연결 (HookDefinition)', []],
      ['sys-migration', '마이그레이션', '임베딩 재생성 · 벡터DB 재구축 · 전체 재가공', []],
    ];

    // 네비게이션
    let navHtml = '';
    groups.forEach(([id, label, desc], i) => {
      navHtml += `<button class="settings-nav-item${i === 0 ? ' active' : ''}" data-action="settings-nav" data-section="${id}">
        <span class="nav-label">${label}</span>
        <span class="nav-desc">${desc}</span>
      </button>`;
    });
    nav.innerHTML = navHtml;

    // 콘텐츠 — 그룹별 섹션 렌더링
    let html = '';
    groups.forEach(([groupId, groupLabel, , sectionKeys], gi) => {
      const hidden = gi === 0 ? '' : ' hidden';
      html += `<div class="settings-group${hidden}" id="settings-section-${groupId}">`;

      // 크레덴셜 관리 그룹
      if (groupId === 'credentials') {
        html += `<div class="settings-section">
          <h3>LLM 크레덴셜</h3>
          <p class="section-guide">LLM 프로바이더(Claude CLI, Anthropic API, OpenAI, Ollama, Gemini)의 인증 정보를 등록합니다. 등록된 크레덴셜은 파이프라인 스텝에서 선택할 수 있습니다.</p>
          <div id="settings-credentials-inline"></div>
          <button class="btn btn-secondary" style="margin-top:var(--spacing-sm)" data-action="new-credential">+ 새 크레덴셜</button>
        </div>`;
      }



      // 일반 그룹: 대시보드/로깅/알림을 3col 그리드로 표시
      if (groupId === 'sys-general') {
        const systemCols = ['logging'];
        html += '<div class="system-3col">';
        for (const sk of systemCols) {
          const fields = meta[sk];
          if (!fields) continue;
          const label = {'logging':'로깅'}[sk]||sk;
          html += `<div class="system-col"><h3 class="section-toggle" style="color:var(--color-primary);font-size:.9rem;margin-bottom:var(--spacing-sm)" data-action="toggle-section">${label}</h3><div class="section-body">`;
          for (const [field, info] of Object.entries(fields)) {
            const value = this._getNestedValue(config, sk, field);
            const input = this._renderField(sk, field, info, value);
            const searchText = `일반 ${sk} ${field} ${info.description}`.toLowerCase();
            html += `<div class="settings-field" data-search="${searchText}" style="padding:4px 0"><div class="settings-field-header"><label>${field}</label></div><div class="field-help">${info.description}</div>${input}</div>`;
          }
          html += '</div></div>';
        }
        html += '</div>';
      }

      // 운영 그룹 (Phase 100): 5 운영 카드를 settings-ops-cards 컨테이너로 위임
      // settings-ops-cards는 index.html 별도 위치에 있고, 운영 그룹 활성 시점에 본 위치로 이동
      if (groupId === 'sys-ops') {
        html += `<div id="settings-ops-cards-mount"></div>`;
      }

      // 해당 그룹의 설정 섹션들 — 멀티컬럼 그리드
      const sectionLabels = {
        'preprocessing':'전처리', 'compression':'압축', 'sensitive':'민감 문서',
        'vector_db':'벡터 DB', 'verification':'검증 설정',
        'logging':'로깅', 'schedule':'스케줄',
        'paths':'경로 설정', 'max_workers':'동시성', 'chunking':'청킹',
        'crossref':'교차참조', 'rerank':'리랭킹',
      };
      const sectionGuides = {
        'schedule': '만료 파일 삭제 및 정합성 검사 주기를 설정합니다.',
        'paths': '추가 감시 폴더를 등록합니다.',
        'max_workers': '파일 동시 처리 워커 수를 설정합니다.',
        'chunking': '대용량 파일(>40KB)을 의미 단위로 분할하여 가공합니다.',
        'crossref': '문서 간 교차참조 관계를 자동 생성합니다.',
        'rerank': '검색 결과를 LLM 기반으로 재정렬합니다.',
      };

      // 일반 그룹의 로깅/알림은 이미 3col로 렌더됨
      const colKeys = sectionKeys.filter(sk => !(groupId === 'sys-general' && ['logging'].includes(sk)));
      if (colKeys.length > 0) {
        // 2개 이상이면 멀티컬럼 그리드
        if (colKeys.length >= 2) html += '<div class="system-3col">';
        for (const sk of colKeys) {
          const fields = meta[sk];
          if (!fields) continue;
          const label = sectionLabels[sk] || sk;
          const guide = sectionGuides[sk] || '';
          html += `<div class="system-col" id="settings-subsection-${sk.replace(/\./g,'-')}"><h3 class="section-toggle" style="color:var(--color-primary);font-size:.9rem;margin-bottom:var(--spacing-sm)" data-action="toggle-section">${label}</h3><div class="section-body">`;
          if (guide) html += `<p class="section-guide" style="font-size:.72rem;margin-bottom:var(--spacing-sm)">${guide}</p>`;
          // 경로 설정: 기본 inbox 읽기전용 표시
          if (sk === 'paths') {
            const basePath = config.paths?.inbox || config.paths?.base || '(exe 디렉토리)/inbox';
            html += `<div class="settings-field" style="padding:4px 0">
              <div class="settings-field-header"><label>inbox (기본)</label></div>
              <div class="field-help">기본 감시 폴더 (자동 설정)</div>
              <input type="text" value="${this._escHtml(basePath)}" readonly style="opacity:.6;cursor:default">
            </div>`;
          }
          // 활성화(boolean) 필드를 맨 위로 정렬
          const fieldEntries = Object.entries(fields);
          const sorted = fieldEntries.sort(([,a],[,b]) => {
            const aEnabled = a.field_type === 'boolean' && /enabled|활성/.test(a.description);
            const bEnabled = b.field_type === 'boolean' && /enabled|활성/.test(b.description);
            return bEnabled - aEnabled;
          });
          for (const [field, info] of sorted) {
            const value = this._getNestedValue(config, sk, field);
            const input = this._renderField(sk, field, info, value);
            const badge = info.requires_restart ? '<span class="restart-badge">restart</span>' : '';
            const searchText = `${groupLabel} ${sk} ${field} ${info.description}`.toLowerCase();
            html += `<div class="settings-field" data-search="${searchText}" style="padding:4px 0">
              <div class="settings-field-header"><label>${field}</label>${badge}</div>
              <div class="field-help">${info.description}</div>
              ${input}
            </div>`;
          }
          html += '</div></div>'; // section-body + system-col
        }
        if (colKeys.length >= 2) html += '</div>';
      }
      // 인프라 그룹 맨 끝에 마이그레이션 패널 추가
      if (groupId === 'sys-migration') {
        html += renderMigrationPanel();
      }
      // 이벤트 훅 그룹 — HookDefinition CRUD
      if (groupId === 'sys-hooks') {
        const hooks = config.hooks || [];
        html += `<div class="settings-section">
          <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:var(--spacing-sm)">
            <h3 style="margin:0">이벤트 훅 (HookDefinition)</h3>
            <button class="btn btn-primary" data-action="add-hook" style="font-size:.72rem;padding:2px 8px">+ 훅 추가</button>
          </div>
          <p class="section-guide">가공 단계 이벤트(file_detected / process_start / process_complete / verify_fail / search_query)에 외부 명령(쉘/HTTP)을 연결합니다.</p>
          <p class="field-help" style="color:var(--color-text-dim);font-size:.78rem;margin-bottom:var(--spacing-md)">
            현재 등록된 훅: <strong style="color:var(--color-primary)">${hooks.length}건</strong>
          </p>`;
        if (hooks.length > 0) {
          html += `<table class="doc-table" style="width:100%"><thead><tr><th style="width:8em">활성</th><th style="width:12em">이벤트</th><th>대상 (URL 또는 명령)</th><th style="width:8em">조치</th></tr></thead><tbody>`;
          hooks.forEach((h, idx) => {
            const target = h.webhook_url || h.command || '';
            const targetType = h.webhook_url ? 'HTTP' : (h.command ? '명령' : '-');
            const enabledBadge = h.enabled
              ? '<span style="color:var(--color-success)">✓ ON</span>'
              : '<span style="color:var(--color-text-dim)">○ OFF</span>';
            html += `<tr>
              <td>${enabledBadge}</td>
              <td><code>${this._escHtml(h.event || '?')}</code></td>
              <td><span style="color:var(--color-text-dim);font-size:.7rem;margin-right:6px">${targetType}</span><code style="font-size:.72rem">${this._escHtml(target)}</code></td>
              <td style="white-space:nowrap">
                <button class="btn btn-sm" data-action="edit-hook" data-index="${idx}" style="font-size:.7rem;padding:2px 6px">편집</button>
                <button class="btn btn-sm btn-secondary" data-action="delete-hook" data-index="${idx}" style="font-size:.7rem;padding:2px 6px;margin-left:4px">삭제</button>
              </td>
            </tr>`;
          });
          html += `</tbody></table>`;
        } else {
          html += `<p style="color:var(--color-text-dim);font-style:italic;font-size:.82rem">등록된 훅이 없습니다. [+ 훅 추가]를 눌러 시작하세요.</p>`;
        }
        html += `</div>`;
      }
      html += '</div>';
    });
    form.innerHTML = html;

    // 크레덴셜 로드 (크레덴셜 관리 그룹에서 사용)
    this._loadInlineCredentials();

    // Phase 100: 운영 카드를 sys-ops 그룹 mount 위치로 이동 (단 1회, 초기 렌더 시점)
    this._mountOpsCards();
  },

  // Phase 100: settings-ops-cards 컨테이너를 sys-ops 그룹의 settings-ops-cards-mount 위치로 이동
  // 좌측 네비 "운영" 클릭 시 sys-ops 그룹만 표시 → 본 컨테이너가 운영 콘텐츠로 나타남
  _mountOpsCards() {
    const cards = document.getElementById('settings-ops-cards');
    const mount = document.getElementById('settings-ops-cards-mount');
    if (cards && mount && cards.parentNode !== mount) {
      mount.appendChild(cards);
      cards.style.display = ''; // 운영 그룹 표시 시점에 자동 노출 (그룹 hidden 토글이 부모 단위)
    }
  },



  // ── Pipeline Builder는 vm.pb 네임스페이스 (아래에 정의) ──

  async _loadInlineCredentials() {
    const container = document.getElementById('settings-credentials-inline');
    if (!container) return;
    const data = await API.listCredentials();
    const creds = data.credentials || [];
    this.state._cachedCredentials = creds;
    const providerLabels = { claude_cli:'Claude CLI', ollama:'Ollama', anthropic_api:'Anthropic', openai_api:'OpenAI', gemini:'Gemini' };
    const defaultCred = this.state.config?.llm?.default_credential || null;
    if (!creds.length) {
      container.innerHTML = '<p style="color:var(--color-text-muted);font-size:.85rem">등록된 크레덴셜이 없습니다.</p>';
    } else {
      const providerIcons = { claude_cli:'🤖', anthropic_api:'🅰️', openai_api:'🟢', ollama:'🦙', gemini:'💎' };
      container.innerHTML = `<div class="cred-card-grid">${creds.map(c => {
        const isDefault = c.name === defaultCred;
        const icon = providerIcons[c.provider] || '🔧';
        return `<div class="cred-card ${isDefault ? 'cred-card-default' : ''}">
          <div class="cred-card-header">
            <span class="cred-card-icon">${icon}</span>
            <span class="cred-card-name">${this._escHtml(c.name)}</span>
            ${isDefault ? '<span class="cred-default-badge">✓ 기본</span>' : ''}
          </div>
          <div class="cred-card-body">
            <div class="cred-card-field"><span class="cred-card-label">프로바이더</span><span>${providerLabels[c.provider]||c.provider}</span></div>
            ${c.model ? `<div class="cred-card-field"><span class="cred-card-label">모델</span><span>${this._escHtml(c.model)}</span></div>` : ''}
            <div class="cred-card-field"><span class="cred-card-label">API 키</span><span>${c.has_api_key ? '••••••••' : '—'}</span></div>
          </div>
          <div class="cred-card-actions">
            ${!isDefault ? `<button class="btn btn-secondary" data-action="set-default-credential" data-name="${this._escHtml(c.name)}">기본으로 설정</button>` : ''}
            <button class="btn btn-secondary" data-action="edit-credential" data-cred-id="${c.id}">수정</button>
            <button class="btn btn-secondary" style="color:var(--color-error)" data-action="delete-credential" data-name="${this._escHtml(c.name)}">삭제</button>
          </div>
        </div>`;
      }).join('')}</div>`;
    }
  },

  // provider별 기본 모델 목록
  _defaultModels: {
    claude_cli: ['sonnet','opus','haiku'],
    anthropic_api: ['claude-sonnet-4-20250514','claude-opus-4-20250514','claude-haiku-4-5-20251001'],
    openai_api: ['gpt-4o','gpt-4o-mini','gpt-4-turbo','o3-mini'],
    ollama: ['llama3','llama3.1','mistral','codellama','gemma2'],
    gemini: ['gemini-2.0-flash','gemini-2.5-pro','gemini-2.5-flash'],
  },
  _providerLabels: { claude_cli:'Claude CLI', ollama:'Ollama', anthropic_api:'Anthropic', openai_api:'OpenAI', gemini:'Gemini' },

  settingsFilterBySearch(query) {
    const q = query.toLowerCase().trim();
    let totalVisible = 0;

    // 검색어 없으면 네비게이션 active 그룹만 표시
    if (!q) {
      const active = document.querySelector('.settings-nav-item.active');
      if (active) { this.settingsScrollTo(active.dataset.section); return; }
    }

    // 검색어 있으면 모든 그룹 보이게 하고 필드 필터링
    document.querySelectorAll('.settings-nav-item').forEach(n => n.classList.remove('active'));
    document.querySelectorAll('.settings-group').forEach(g => g.classList.remove('hidden'));
    document.querySelectorAll('.settings-section, .system-col').forEach(sec => {
      const fields = sec.querySelectorAll('.settings-field');
      let sectionVisible = 0;
      fields.forEach(f => {
        if (!f.dataset.search) return;
        const match = f.dataset.search.includes(q);
        f.classList.toggle('hidden', !match);
        f.classList.toggle('search-highlight', match && !!q);
        if (match) sectionVisible++;
      });
      sec.classList.toggle('hidden', sectionVisible === 0);
      // 접힌 섹션이면 매칭 시 자동 펼치기
      const toggle = sec.querySelector('.section-toggle');
      if (toggle && sectionVisible > 0 && q) toggle.classList.remove('collapsed');
      totalVisible += sectionVisible;
    });

    // 결과 없음 표시
    let noResults = document.getElementById('settings-no-results');
    if (totalVisible === 0 && q) {
      if (!noResults) {
        noResults = document.createElement('div');
        noResults.id = 'settings-no-results';
        noResults.className = 'settings-no-results';
        document.getElementById('settings-form').appendChild(noResults);
      }
      noResults.textContent = `"${query}"에 해당하는 설정이 없습니다.`;
      noResults.style.display = '';
    } else if (noResults) {
      noResults.style.display = 'none';
    }
  },

  settingsScrollTo(section) {
    Modal.close();
    // 선택한 그룹만 표시, 나머지 숨김
    document.querySelectorAll('.settings-group').forEach(g => {
      g.classList.toggle('hidden', g.id !== 'settings-section-' + section);
    });
    document.querySelectorAll('.settings-nav-item').forEach(n => n.classList.toggle('active', n.dataset.section === section));
    const search = document.getElementById('settings-search');
    if (search) search.value = '';
    // 그룹 내 모든 필드 표시
    document.querySelectorAll('.settings-field').forEach(f => f.classList.remove('hidden'));
    document.querySelectorAll('.settings-section').forEach(s => s.classList.remove('hidden'));
  },

  _getNestedValue(config, section, field) {
    // 최상위 필드: section과 field가 같고, config[section]이 객체가 아닌 경우
    if (section === field && config && typeof config[section] !== 'object') {
      return config[section];
    }
    const parts = section.split('.');
    let obj = config;
    for (const p of parts) { obj = obj && obj[p]; }
    if (field.includes('.')) {
      const fp = field.split('.');
      for (const p of fp) { obj = obj && obj[p]; }
      return obj;
    }
    return obj && obj[field];
  },

  _renderField(section, field, info, value) {
    const id = `cfg-${section}-${field}`.replace(/\./g, '-');
    const ds = `data-section="${section}" data-field="${field}"`;
    const ft = info.field_type;

    // select:value1=label1|value2=label2 형식
    if (ft.startsWith('select:')) {
      const options = ft.substring(7).split('|').map(opt => {
        const [val, label] = opt.split('=');
        const selected = String(value) === val ? ' selected' : '';
        return `<option value="${val}"${selected}>${label || val}</option>`;
      });
      return `<select id="${id}" ${ds}>${options.join('')}</select>`;
    }

    switch (ft) {
      case 'boolean':
        return `<input type="checkbox" id="${id}" ${value ? 'checked' : ''} ${ds}>`;
      case 'integer':
        return `<input type="number" id="${id}" value="${value ?? ''}" step="1" ${ds}>`;
      case 'float':
        return `<input type="number" id="${id}" value="${value ?? ''}" step="0.01" ${ds}>`;
      case 'string_array': {
        const items = value || [];
        let tableHtml = `<div class="array-table" id="${id}" ${ds} data-array="true">`;
        items.forEach((item, i) => {
          tableHtml += `<div class="array-row"><input type="text" value="${this._escHtml(item)}" data-array-idx="${i}" style="flex:1"><button type="button" class="btn btn-secondary" style="font-size:.7rem;padding:1px 6px;color:var(--color-error)" onclick="this.parentElement.remove();document.getElementById('${id}').dispatchEvent(new Event('change'))">✕</button></div>`;
        });
        tableHtml += `<button type="button" class="btn btn-secondary" style="font-size:.72rem;margin-top:4px" onclick="const row=document.createElement('div');row.className='array-row';row.innerHTML='<input type=\\'text\\' value=\\'\\'style=\\'flex:1\\'><button type=\\'button\\' class=\\'btn btn-secondary\\' style=\\'font-size:.7rem;padding:1px 6px;color:var(--color-error)\\' onclick=\\'this.parentElement.remove();document.getElementById(&quot;${id}&quot;).dispatchEvent(new Event(&quot;change&quot;))\\'>✕</button>';this.before(row);row.querySelector('input').focus()">+ 추가</button></div>`;
        return tableHtml;
      }
      case 'secret':
        return `<div style="position:relative"><input type="password" id="${id}" value="${value ?? ''}" placeholder="비어있으면 비활성" ${ds} style="padding-right:32px"><button type="button" class="eye-toggle" onclick="const i=this.previousElementSibling;i.type=i.type==='password'?'text':'password';this.textContent=i.type==='password'?'👁':'👁‍🗨'" style="position:absolute;right:4px;top:50%;transform:translateY(-50%);background:none;border:none;cursor:pointer;font-size:.8rem;color:var(--color-text-dim)">👁</button></div>`;
      default:
        return `<input type="text" id="${id}" value="${value ?? ''}" ${ds}>`;
    }
  },

  _collectSettings() {
    const config = JSON.parse(JSON.stringify(this.state.config));
    document.querySelectorAll('#settings-form [data-section]').forEach(el => {
      const section = el.dataset.section;
      const field = el.dataset.field;
      let value;
      if (el.type === 'checkbox') value = el.checked;
      else if (el.type === 'number') value = el.step === '1' ? parseInt(el.value) || 0 : parseFloat(el.value) || 0;
      else if (el.tagName === 'SELECT') value = isNaN(el.value) ? el.value : parseFloat(el.value);
      else if (el.dataset.array) {
        // array-table: div 안에 여러 input
        if (el.classList.contains('array-table')) {
          value = Array.from(el.querySelectorAll('.array-row input')).map(inp => inp.value.trim()).filter(Boolean);
        } else {
          value = el.value.split(',').map(s => s.trim()).filter(Boolean);
        }
      }
      else value = el.value;

      // secret 필드: "****"이면 변경하지 않음
      if (el.type === 'password' && value === '****') return;

      // 최상위 필드: section과 field가 같고, 기존 값이 객체가 아닌 경우
      if (section === field && typeof config[section] !== 'object') {
        config[field] = value;
        return;
      }

      // 중첩 경로 처리 (section: "verification.thresholds", field: "structure_min")
      const sectionParts = section.split('.');
      let obj = config;
      for (const p of sectionParts) {
        if (!obj[p]) obj[p] = {};
        obj = obj[p];
      }

      // field도 중첩 가능 (notification → telegram.bot_token)
      if (field.includes('.')) {
        const fp = field.split('.');
        let target = obj;
        for (let i = 0; i < fp.length - 1; i++) {
          if (!target[fp[i]]) target[fp[i]] = {};
          target = target[fp[i]];
        }
        target[fp[fp.length - 1]] = value || null;
      } else {
        obj[field] = value;
      }
    });
    return config;
  },

  async saveSettings() {
    const config = this._collectSettings();
    const result = await API.updateConfig(config);
    const status = document.getElementById('settings-status');
    if (result.errors) {
      status.innerHTML = `<div class="status-error">${result.errors.join('<br>')}</div>`;
    } else if (result.restart_required) {
      status.innerHTML = '<div class="status-warning">저장 완료. 일부 설정은 재시작 후 적용됩니다.</div>';
      this.state.config = config;
    } else {
      status.innerHTML = '<div class="status-success">저장 완료. 설정이 즉시 반영되었습니다.</div>';
      this.state.config = config;
    }
    setTimeout(() => { status.innerHTML = ''; }, 5000);
  },

  async importConfig() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.toml';
    input.onchange = async () => {
      const file = input.files[0];
      if (!file) return;
      const content = await file.text();
      const result = await API.importConfigToml(content);
      if (result.errors) {
        this._showMsg('설정 검증 실패: ' + result.errors.join(', '), 'error');
        return;
      }
      if (result.ok) {
        this._showMsg('pipeline.toml 가져오기 완료. 설정을 다시 로드합니다.', 'success');
        this.state.config = null;
        this.state.configMeta = null;
        this.loadSettings();
      }
    };
    input.click();
  },

  async exportConfig() {
    const result = await API.exportConfigToml();
    if (!result.toml) {
      this._showMsg('설정 내보내기 실패', 'error');
      return;
    }
    const blob = new Blob([result.toml], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'pipeline.toml';
    a.click();
    URL.revokeObjectURL(url);
    this._showMsg('pipeline.toml 내보내기 완료', 'success');
  },

  // 초기화
  async init() {
    this.state.stats = await API.stats();
    this.state.kgStats = await API.kgStats();
    this.state._tokenUsage = await API.tokenUsage();
    this.state._llmCacheStats = await API.getLlmCacheStats().catch(() => ({}));
    // watcher 상태 초기화
    const ws = await API.watcherStatus();
    this._updateWatcherToggle(ws.active !== false);
    this.renderStats();
    // 기본 탭: 문서 목록
    this.switchTab('documents');
    this.loadDocuments();
    // doc_type 필터 옵션 동적 생성
    if (this.state.stats && this.state.stats.by_type) {
      const sel = document.getElementById('doc-type-filter');
      this.state.stats.by_type.forEach(([type]) => {
        const opt = document.createElement('option');
        opt.value = type; opt.textContent = type;
        sel.appendChild(opt);
      });
    }

    // 5초마다 자동 갱신 (stats + queue)
    setInterval(() => this.refreshDashboard(), 5000);

    // credential 0건 시 온보딩 자동 시작 (release 환경 첫 실행 케어).
    // dev 환경은 백엔드 dev_seed_credential이 in-memory로 1건 주입하므로 자동 트리거 안 됨.
    try {
      const creds = await API.listCredentials();
      if (!creds.credentials || creds.credentials.length === 0) {
        setTimeout(() => this.startOnboarding(), 300);
      }
    } catch (e) {
      console.warn('credential 조회 실패, 온보딩 자동 시작 skip:', e);
    }
  },

  async refreshDashboard() {
    try {
      const [stats, kgStats, tokenUsage, qData, cacheStats, progressData] = await Promise.all([
        API.stats(), API.kgStats(), API.tokenUsage(), API.queue(),
        API.getLlmCacheStats().catch(() => ({})),
        API.progress().catch(() => ({ events: [] })),
      ]);
      this.state.stats = stats;
      this.state.kgStats = kgStats;
      this.state._tokenUsage = tokenUsage;
      this.state._llmCacheStats = cacheStats;
      // get_queue 응답 구조: { stats: {total, pending, processing, done, failed}, items: [] }
      const qs = qData.stats || qData;
      this.state._queueStats = {
        processing: qs.processing || 0,
        pending: qs.pending || 0,
        failed: qs.failed || 0,
        done: qs.done || 0,
      };
      this.renderStats();
      if (this.state.activeTab === 'documents') this.loadDocuments();
      if (this.state.activeTab === 'processing') this.loadProcessing();

      // R-2c: progress 채널 start 이벤트 도착 시 inbox 감지 → 다음 tick에서 즉시 재갱신.
      // 5초 폴링 대기 없이 사용자에게 inbox 감지를 빠르게 보여줌. done/error도 함께 트리거.
      const events = progressData.events || [];
      const seen = this.state._lastProgressEventIds || new Set();
      let hasNewActivity = false;
      for (const ev of events) {
        if (!seen.has(ev)) {
          hasNewActivity = true;
          seen.add(ev);
        }
      }
      // 셋이 무한 성장하지 않도록 100건 초과 시 잘라냄
      if (seen.size > 100) {
        this.state._lastProgressEventIds = new Set(Array.from(seen).slice(-100));
      } else {
        this.state._lastProgressEventIds = seen;
      }
      if (hasNewActivity) {
        // 다음 tick에 한 번 더 즉시 갱신 (이벤트 직후 큐 상태가 변했을 가능성)
        setTimeout(() => this._refreshQueueOnly(), 200);
      }
    } catch(e) {}
  },

  // queue stats만 가볍게 갱신 (progress 이벤트 도착 직후 즉시 재반영용).
  async _refreshQueueOnly() {
    try {
      const qData = await API.queue();
      const qs = qData.stats || qData;
      this.state._queueStats = {
        processing: qs.processing || 0,
        pending: qs.pending || 0,
        failed: qs.failed || 0,
        done: qs.done || 0,
      };
      this.renderStats();
    } catch(e) {}
  },


  _updateWatcherToggle(active) {
    const btn = document.getElementById('watcher-toggle');
    if (!btn) return;
    if (active) {
      btn.classList.add('active');
      btn.querySelector('.watcher-label').textContent = '감지 ON';
    } else {
      btn.classList.remove('active');
      btn.querySelector('.watcher-label').textContent = '감지 OFF';
    }
    this.state._watcherActive = active;
  },

  async _toggleWatcher() {
    const newState = !this.state._watcherActive;
    const result = await API.setWatcherActive(newState);
    if (result.ok) {
      this._updateWatcherToggle(result.active);
      this._showMsg(newState ? 'inbox 감지 활성화' : 'inbox 감지 일시 정지', newState ? 'success' : 'warning');
    }
  },

  _showMsg(msg, type) {
    // settings-status 또는 body에 표시
    let el = document.getElementById('settings-status');
    if (!el) {
      el = document.createElement('div');
      el.id = 'settings-status';
      document.body.prepend(el);
    }
    const cls = type === 'success' ? 'status-success' : type === 'error' ? 'status-error' : 'status-warning';
    el.innerHTML = `<div class="${cls}">${msg}</div>`;
    setTimeout(() => { el.innerHTML = ''; }, 5000);
  },

  // ══════════════════════════════════════════════════════════════
  // ── Pipeline Builder (Ansible Tower 스타일) ──
  // ══════════════════════════════════════════════════════════════

  pb: {
    pipelines: [],
    selectedIdx: 0,
    selectedNodeId: null,
    selectedInspector: null,  // Phase 66: 인스펙터에 표시할 노드 (process 노드ID / "search:xxx" / "batch:xxx")
    activeMidTab: 'process',  // Phase 66: 가운데 3탭 활성 상태
    groupCollapsed: {},
    simInput: '',
    simMode: 'text',
    simResults: null,
  },

  // 노드 타입 정의 (19 fixed steps)
  PB_NODES: {
    // Phase 66: 공통 전처리 첫 노드 — 입력 소스
    input_source: { label:'입력 소스', group:'precheck', always:true, icon:'📂',
      desc:'파이프라인이 감시할 폴더 목록.',
      why:'기본 inbox 외에 여러 폴더를 동시에 감시 가능. 등록된 모든 폴더에 동일 파이프라인 적용.',
      how:'paths.extra_inboxes에 절대 경로를 등록. 기본 inbox는 auto_init이 (exe 디렉토리)/inbox로 자동 설정.',
      configSections:['paths'],
      fields:[]},
    // 공통 전처리 (1-4)
    sensitive: { label:'민감 판별', group:'precheck', always:true, icon:'🔒',
      desc:'파일명/내용에서 민감 키워드를 탐지합니다.',
      why:'개인정보, API 키 등이 외부 LLM에 전송되는 것을 방지',
      how:'SensitivityDetector가 키워드(keywords)와 확장자(extensions) 기반으로 판별. 민감 파일은 sensitive/ 폴더로 이동.',
      configSections:['sensitive'],
      fields:[]},
    fragment: { label:'Fragment 감지', group:'precheck', always:true, icon:'📋',
      desc:'짧은 메모를 감지하여 LLM을 스킵합니다.',
      why:'100자 이하 짧은 메모에 LLM 호출은 비용 낭비',
      how:'fragment_threshold(기본 100) 이하 글자수 파일은 LLM 분류/가공, Verify, Embedding 모델오버라이드, Storage 레벨오버라이드를 모두 스킵하고 원본 텍스트를 직접 벡터DB에 색인합니다.',
      // Phase 69: schedule.fragment_threshold만 추출해 노출
      configFields:[['schedule','fragment_threshold']],
      fields:[]},
    sha256: { label:'SHA-256 중복', group:'precheck', always:true, icon:'#️⃣',
      desc:'파일의 바이트 단위 완전 동일 여부를 확인합니다.',
      why:'동일 파일 반복 투입 시 LLM 호출 비용 낭비 방지. 이 체크는 비활성화할 수 없습니다.',
      how:'SHA-256 해시를 계산하여 벡터DB에 동일 해시가 있으면 스킵. 의미 중복 체크(cosine similarity)는 후처리에서 별도 실행되며 semantic_dup_threshold 설정의 영향을 받습니다.',
      fields:[]},
    incremental: { label:'증분 해시', group:'precheck', always:true, icon:'🔄',
      desc:'이전 처리 이후 파일이 변경되었는지 확인합니다.',
      why:'변경되지 않은 파일의 재처리 방지',
      how:'CompileState에 파일별 해시를 기록. 해시가 동일하면 스킵.',
      fields:[]},

    // 파이프라인 스텝 (5-9)
    preprocess: { label:'Preprocess', group:'step', always:true, icon:'📄',
      desc:'비텍스트 파일(PDF, DOCX, XLSX, PPTX, 이미지)을 텍스트로 변환합니다.',
      why:'LLM은 텍스트만 처리 가능. 바이너리 파일을 텍스트로 변환해야 가공 가능.',
      how:'확장자별 호스트에 설치된 도구를 자동 감지하여 사용. 수동 선택도 가능.',
      configSections:['preprocessing'],
      fields:[]},
    llm: { label:'LLM 분류+가공', group:'step', always:true, icon:'🤖',
      desc:'문서를 분류하고 구조화된 형식으로 가공합니다.',
      why:'원본 텍스트를 검색 가능한 구조(유형/키워드/섹션/요약)로 변환',
      how:'분류와 가공을 단일 LLM 호출(classify_and_process)로 동시 수행합니다. 대용량 파일은 Chunking 노드 산출물을 받아 청크별 가공 후 병합. 실패 시 max_retry만큼 재시도.',
      configSections:['models'],
      fields:[
        { key:'credential', type:'credential', label:'크레덴셜', desc:'분류+가공에 사용할 LLM' },
        { key:'prompt', type:'prompt_editor', label:'프롬프트 템플릿', desc:'분류+가공 프롬프트를 편집합니다. prompts.toml에 저장되며 즉시 반영됩니다.' },
      ]},
    verify: { label:'Verify', group:'step', always:true, icon:'✅',
      desc:'가공 결과의 품질을 6가지 기준으로 검증합니다.',
      why:'LLM 환각/누락 방지. 구조 완전성, 키워드 커버리지, ROUGE-L 등 자동 검증.',
      how:'검증 실패 시 피드백을 포함하여 2-Pass 재가공. 2차도 실패하면 quarantine/ 폴더로 이동.',
      configSections:['verification', 'verification.thresholds'],
      fields:[
        { key:'enabled', type:'checkbox', label:'검증 활성화', default:true },
        { key:'credential', type:'credential', label:'검증 크레덴셜', desc:'2-Pass 재가공 시 사용할 LLM (미지정 시 분류/가공 LLM 사용)' },
      ]},
    // Phase 67: Chunking 노드 신규 (LLM 입력 단위 조정)
    chunking: { label:'Chunking', group:'step', always:true, icon:'✂',
      desc:'대용량 파일(>40KB)을 의미 단위로 분할하여 각 청크를 별도 가공 후 병합합니다.',
      why:'LLM 입력 토큰 한도 + 의미 단위 보존으로 가공 품질 향상',
      how:'헤딩(H1/H2/H3) 인식 후 target_bytes 기준 분할. 코드펜스(```) 보존. 청크 간 overlap_sentences 만큼 중첩.',
      configSections:['chunking'],
      fields:[]},
    // embedding 스텝은 embed_gen(후처리)과 통합됨 — 설정은 embed_gen에서 관리
    // storage 스텝은 save_compress(후처리)와 통합됨 — 설정은 save_compress에서 관리

    // 공통 후처리 (Phase 67 정렬: Embedding → 의미 중복 → 저장압축 → 원격 업로드 → 벡터DB → 교차참조 → 엔티티 → Todo → Topic → 알림 → 증분 기록)
    embed_gen: { label:'임베딩 생성', group:'post', always:true, icon:'🧮',
      desc:'가공된 텍스트를 벡터로 변환하여 유사도 검색을 가능하게 합니다.',
      why:'텍스트를 수치 벡터로 변환해야 코사인 유사도 검색과 벡터DB 색인이 가능',
      how:'fastembed BGE-M3 (1024차원, 순수 Rust, 로컬 고정). 모델은 첫 실행 시 자동 다운로드.',
      // Phase 65/67: fastembed 고정. UI 비노출.
      fields:[]},
    semantic_dup: { label:'의미 중복', group:'post', always:true, icon:'🔍',
      desc:'코사인 유사도로 의미적 중복 문서를 탐지합니다.',
      why:'SHA-256은 바이트 동일만 감지. 의미적으로 같은 내용의 다른 파일을 탐지.',
      how:'벡터DB에서 가장 유사한 문서를 찾아 semantic_dup_threshold 이상이면 중복으로 판정. 정책: Stub Keep(둘 다 유지).',
      configSections:['vector_db'],
      fields:[]},
    save_compress: { label:'저장+압축', group:'post', always:true, icon:'📦',
      desc:'가공본과 원본을 zstd 압축하여 디스크에 저장합니다. .vec 임베딩 파일도 함께 저장.',
      why:'processed/, originals/ 폴더에 압축 파일로 영속화',
      how:'zstd 알고리즘으로 압축. 레벨 1(빠름)~22(최대 압축). 기본값 3.',
      configSections:['compression'],
      fields:[]},
    remote_upload: { label:'원격 업로드', group:'post', always:true, icon:'☁',
      desc:'가공본/원본을 외부 저장소에 백업합니다.',
      why:'로컬 장애 시 복구 가능, NAS/클라우드 자동 백업',
      how:'remote_storage 설정이 활성화되면 processed/와 originals/를 업로드. 실패 시 로그만 남기고 처리 계속.',
      configSections:['remote_storage'],
      fields:[]},
    vectordb: { label:'벡터DB 색인', group:'post', always:true, icon:'🗄',
      desc:'문서를 벡터DB에 색인하여 검색 가능하게 합니다.',
      why:'dense(의미) + sparse(키워드 BM25) 듀얼 벡터로 하이브리드 검색 지원',
      how:'LocalVectorStore(인프로세스)에 upsert. HNSW 인덱스 + 키워드 역색인.',
      // Phase 69: vector_db에서 색인·검색 공통 필드 노출
      configFields:[['vector_db','search_top_k'], ['vector_db','rrf_multiplier']],
      fields:[]},
    crossref: { label:'교차참조', group:'post', always:true, icon:'🔗',
      desc:'문서 간 관련성을 탐지하여 5종 관계 링크를 생성합니다.',
      why:'지식 그래프 구축. 관련 문서를 자동으로 연결.',
      how:'비동기 큐 → 30초 유휴 시 배치 flush. EmbeddingSnapshot 행렬곱 + threshold 판정 (Supersedes/Updates/RelatedTopic/References/ReferencedBy). MinHash LSH + 메타블로킹 옵션.',
      configSections:['crossref'],
      fields:[]},
    entity_extract: { label:'엔티티 추출', group:'post', always:true, icon:'🏷',
      desc:'문서에서 사람/조직/기술/프로젝트 등 엔티티를 추출합니다.',
      why:'엔티티 정보를 벡터DB에 색인하여 검색 품질 향상',
      how:'LLM 가공 시 entities 필드로 추출 (우선). LLM이 미반환 시 regex 폴백 (날짜/금액/숫자/이메일/URL).',
      fields:[]},
    // Phase 67: Todo 병합 노드 신규
    todo_merge: { label:'Todo 병합', group:'post', always:true, icon:'☑',
      desc:'문서에서 체크박스/키워드 7종을 추출해 settings.db todos에 누적.',
      why:'문서를 가공할 때 자동으로 할일 목록을 빌드. 어제 미완료는 오늘로 carry-forward.',
      how:'마크다운 [ ]/[x] + 키워드(TODO/FIXME/HACK/XXX/할일/검토필요/확인바람). fingerprint=SHA-256(정규화)로 중복 방지.',
      fields:[]},
    // Phase 67: 토픽 자동 병합 노드 신규
    topic_merge: { label:'토픽 자동 병합', group:'post', always:true, icon:'🗂',
      desc:'유형별 클러스터가 임계 도달 시 자동으로 토픽 페이지 생성/병합.',
      why:'문서가 누적될수록 주제별 묶음이 자연스럽게 형성. 모순 자동 탐지.',
      how:'auto_merge_threshold 도달 시 임베딩 agglomerative 클러스터링 → LLM 라벨링 → 시간 분할 → 2단계 요약 → 모순 해결.',
      configSections:['preprocessing'],  // auto_merge_threshold + max_topic_chars 위치
      fields:[]},
    notify: { label:'알림', group:'post', always:true, icon:'🔔',
      desc:'처리 완료를 Telegram/Slack으로 알립니다.',
      why:'파일 투입 후 처리 완료 여부를 실시간 확인',
      how:'NotificationPort를 통해 Telegram(HTML)/Slack(mrkdwn) 전송. 배치 요약 알림(30초 유휴 시).',
      configSections:['notification'],
      fields:[]},
    compile_state: { label:'증분 기록', group:'post', always:true, icon:'📅',
      desc:'처리 결과를 증분 상태에 기록합니다.',
      why:'다음 실행 시 변경되지 않은 파일을 스킵하기 위한 상태 기록',
      how:'CompileState에 파일 해시, 크기, 타임스탬프 저장.',
      fields:[]},
    // Phase 71: Memory Tier 갱신 노드 (자동 분류, 임계 노출)
    memory_tier: { label:'Memory Tier', group:'post', always:true, icon:'🌡',
      desc:'문서를 hot/warm/cold/archived로 자동 분류합니다.',
      why:'Purge·검색 우선순위 결정. 자주 접근하는 문서일수록 hot.',
      how:'last_accessed + access_count 기반. hot_days 이내 = hot, warm_days = warm, cold_days = cold, 그 이후 archived.',
      configSections:['memory_tier'],
      fields:[]},
    // Phase 71: Lint 노드 (정합성 검사)
    lint: { label:'Lint', group:'post', always:true, icon:'🩺',
      desc:'orphan/stale/모순/허브편중/비대칭 정합성 검사를 주기적으로 실행합니다.',
      why:'문서 누적에 따른 그래프 불균형 자동 탐지',
      how:'lint_interval_hours 주기로 실행. 0=비활성. doctor CLI 또는 스케줄에서 호출.',
      configFields:[['schedule','lint_interval_hours']],
      fields:[]},
    // Phase 71: Quarantine 분기 노드 (Verify 2-Pass 실패 시 격리)
    // branch_from: Verify 노드에서 FAIL 시에만 진입. 정상 흐름은 이 노드 스킵.
    quarantine: { label:'Quarantine 분기', group:'post', always:true, icon:'⛔',
      branch_from:'verify', branch_condition:'2-Pass 검증 모두 FAIL',
      desc:'Verify 2-Pass 재가공도 실패한 파일을 quarantine/ 폴더로 격리합니다.',
      why:'환각/누락이 심한 가공 결과를 코퍼스에서 분리',
      how:'verification.on_fail=quarantine 시 분기. skip_with_notify면 알림만.',
      configFields:[['verification','on_fail']],
      fields:[]},
  },

  // 그룹 정의
  PB_GROUPS: [
    { id:'precheck', label:'공통 전처리', always:true },
    { id:'step',     label:'파이프라인 스텝', always:true },
    { id:'post',     label:'공통 후처리', always:true },
  ],

  // Phase 71: 24노드 (5 + 4 + 15)
  // 공통 전처리(5) + 스텝(4) + 후처리(15: quarantine + memory_tier + lint 신규)
  _pbCurrentNodes() {
    return [
      'input_source','sensitive','fragment','sha256','incremental',
      'preprocess','chunking','llm','verify',
      'quarantine','embed_gen','semantic_dup','save_compress','remote_upload','vectordb',
      'crossref','entity_extract','todo_merge','topic_merge','memory_tier','lint','notify','compile_state'
    ];
  },

  async loadPipelineBuilder() {
    // Load config for node values (no pipeline list needed)
    if (!this.state.config || !this.state.configMeta) {
      try {
        const data = await API.getConfig();
        this.state.config = data.config || null;
        this.state.configMeta = data.metadata || null;
      } catch(e) {}
    }
    // Load credentials for credential fields
    if (!this.state._cachedCredentials) {
      try {
        const data = await API.listCredentials();
        this.state._cachedCredentials = data.credentials || [];
      } catch(e) { this.state._cachedCredentials = []; }
    }
    this.pb.selectedNodeId = null;
    if (!this.pb.activeMidTab) this.pb.activeMidTab = 'process';
    this._initPBNodeValues();
    this.renderPBSidebar();
    this._renderPBMidTabs();
    this.renderPBCanvas();
    this._renderPBSearchPipeline();
    this._renderPBBatchConfig();
    this._renderPBInspector();
    this._applyPBMidTab();
  },

  // Phase 66: Pipeline 가운데 3탭 (가공/검색/배치) ─────────────────
  _renderPBMidTabs() {
    const el = document.getElementById('pb-midtabs');
    if (!el) return;
    el.innerHTML = PIPELINE_TABS.map(t => {
      const active = t.id === this.pb.activeMidTab ? ' active' : '';
      return `<div class="pb-midtab${active}" data-action="pb-mid-tab" data-midtab="${t.id}">${t.label}</div>`;
    }).join('');
  },

  switchPipelineMidTab(midtab) {
    if (!midtab) return;
    this.pb.activeMidTab = midtab;
    this.pb.selectedNodeId = null;
    this.pb.selectedInspector = null;
    this._renderPBMidTabs();
    this._applyPBMidTab();
    this._renderPBInspector();
  },

  _applyPBMidTab() {
    const tabs = ['process', 'search', 'batch'];
    tabs.forEach(t => {
      const el = document.getElementById('pb-mid-' + t);
      if (el) el.style.display = (t === this.pb.activeMidTab) ? '' : 'none';
    });
  },

  // Phase 68/69/71: 검색 파이프라인 노드별 설정 가능 여부 매핑
  _searchNodeHasSettings(nid) {
    const map = {
      query_sensitive: 'sensitive',
      query_expand: 'search',  // Phase 71: 검색 모드 5종 — 별도 처리 필요 (descMap에서)
      hybrid: 'search',         // sparse_weight
      fuse: 'search',           // time_weight (vector_db에서 search로 이동)
      rerank: 'rerank',
      win: 'search',            // window_lines
      mmr: 'search',            // mmr_lambda
      paging: 'vector_db',
    };
    const sk = map[nid];
    if (!sk) return false;
    const meta = this.state.configMeta || {};
    return !!(meta[sk] && Object.keys(meta[sk]).length > 0);
  },

  // 검색 파이프라인 (3섹션: 공통 전처리 / 매칭 스텝 / 결과 후처리)
  _renderPBSearchPipeline() {
    const host = document.getElementById('pb-search-pipeline');
    if (!host) return;
    const sections = [
      ['공통 전처리 (쿼리 정규화)', [
        ['query_input',     '쿼리 입력',      '🔤'],
        ['query_normalize', '쿼리 정규화',    '✨'],
        ['query_sensitive', '민감 차단',      '🔒'],
        ['query_cache',     '쿼리 캐시 조회', '🗂'],
        ['query_expand',    '쿼리 확장',      '➕'],
      ]],
      ['파이프라인 스텝 (검색·매칭)', [
        ['embed',     'Embed',        '🧮'],
        ['hybrid',    'Hybrid Match', '🔀'],
        ['fuse',      'Fuse (RRF)',   '🪄'],
        ['rerank',    'Rerank',       '🎯'],
      ]],
      ['공통 후처리 (결과 가공·전달)', [
        ['win',       'Sentence Window', '🪟'],
        ['crag',      'CRAG',            '🧠'],
        ['parent',    'Parent Expand',   '⬆'],
        ['mmr',       'MMR 다양성',      '🌈'],
        ['kg_attach', '관련 문서 첨부',  '🔗'],
        ['entity_hl', '엔티티 하이라이트', '🏷'],
        ['paging',    '결과 정렬·페이징',  '📄'],
        ['deliver',   'MCP/UI 전달',     '📤'],
        ['log',       '검색 로그 기록',   '📝'],
        ['notify',    '알림',            '🔔'],
        ['metric',    '품질 메트릭',     '📊'],
      ]],
    ];
    let html = '';
    sections.forEach(([title, nodes]) => {
      html += `<div class="pb-section-block">
        <div class="pb-section-block-title">${title}</div>
        <div class="pb-flow-row">`;
      nodes.forEach(([nid, label, icon], i) => {
        const sel = this.pb.selectedInspector === 'search:' + nid ? ' active' : '';
        const hasSettings = this._searchNodeHasSettings(nid);
        const cogClass = hasSettings ? ' has-settings' : ' auto-only';
        const cogIcon = hasSettings ? '<span class="pb-node-cog" title="설정 가능">⚙</span>' : '';
        if (i > 0) html += '<span class="pb-flow-arrow">→</span>';
        html += `<div class="pb-node${sel}${cogClass}" data-action="pb-inspector-node" data-node-id="search:${nid}" data-node-label="${label}" style="cursor:pointer;padding:6px 10px;background:var(--color-surface);border:1px solid var(--color-border);border-radius:4px;font-size:.78rem">${icon} ${label}${cogIcon}</div>`;
      });
      html += '</div></div>';
    });
    host.innerHTML = html;
  },

  // 배치 설정 (스케줄·동시성 + 보존·Purge)
  _renderPBBatchConfig() {
    const host = document.getElementById('pb-batch-config');
    if (!host || !this.state.configMeta) {
      if (host) host.innerHTML = '<p class="pb-inspector-empty">설정을 불러오는 중...</p>';
      return;
    }
    const meta = this.state.configMeta;
    const config = this.state.config || {};
    // Phase 67: 배치 설정에 토픽 자동 병합 추가 (preprocessing 섹션의 auto_merge_threshold/max_topic_chars)
    // preprocessing 섹션 전체를 노출하지 않고 토픽 관련 필드만 추출하기 위해 임시 메타 사전을 구성
    const topicMeta = {};
    if (meta.preprocessing) {
      ['auto_merge_threshold', 'max_topic_chars'].forEach(k => {
        if (meta.preprocessing[k]) topicMeta[k] = meta.preprocessing[k];
      });
    }
    const groups = [
      ['스케줄·동시성', '정합성 검사 주기 + 동시 처리 워커 수', [
        ['schedule', meta.schedule],
        ['max_workers', meta.max_workers],
      ]],
      ['보존·Purge', '원본 보존 기간 + 만료 파일 자동 정리', [
        ['retention', meta.retention],
      ]],
      ['토픽 자동 병합', '문서 누적 시 자동 클러스터링·라벨링', [
        ['preprocessing', Object.keys(topicMeta).length ? topicMeta : null],
      ]],
    ];
    let html = '';
    groups.forEach(([title, desc, sections]) => {
      html += `<div class="pb-section-block"><div class="pb-section-block-title">${title}</div>
        <p class="field-help" style="margin-bottom:var(--spacing-sm)">${desc}</p>`;
      sections.forEach(([sk, fields]) => {
        if (!fields) return;
        for (const [field, info] of Object.entries(fields)) {
          const value = this._getNestedValue(config, sk, field);
          const input = this._renderField(sk, field, info, value);
          html += `<div class="settings-field" style="padding:4px 0;margin-bottom:var(--spacing-sm)">
            <div class="settings-field-header"><label>${field}</label></div>
            <div class="field-help">${info.description}</div>
            ${input}
          </div>`;
        }
      });
      html += '</div>';
    });
    html += `<div style="display:flex;gap:var(--spacing-sm);margin-top:var(--spacing-md);padding-top:var(--spacing-md);border-top:1px solid var(--color-border)">
      <button class="btn btn-primary" data-action="save-settings">저장</button>
      <button class="btn btn-secondary" data-action="purge-dry-run">Purge Dry Run</button>
      <button class="btn btn-secondary" data-action="purge-execute" style="border-color:var(--color-warning);color:var(--color-warning)">Purge 실행</button>
    </div>
    <div id="batch-purge-result" style="margin-top:var(--spacing-md)"></div>`;
    host.innerHTML = html;
  },

  // Phase 67: 우측 인스펙터 (편집 form 직접 통합)
  _renderPBInspector() {
    const host = document.getElementById('pb-inspector');
    if (!host) return;
    const sel = this.pb.selectedInspector;
    if (!sel) {
      host.innerHTML = `<div class="pb-inspector-empty">노드를 클릭하면 설명과 설정 항목이 여기에 표시됩니다.</div>`;
      return;
    }
    // 검색 파이프라인 노드
    if (sel.startsWith('search:')) {
      host.innerHTML = this._renderSearchInspector(sel.substring(7));
      return;
    }
    // 배치 설정 섹션
    if (sel.startsWith('batch:')) {
      host.innerHTML = this._renderBatchInspector(sel.substring(6));
      return;
    }
    // 가공 파이프라인 노드 (PB_NODES)
    const nodeDef = this.PB_NODES && this.PB_NODES[sel];
    if (!nodeDef) {
      host.innerHTML = `<div class="pb-inspector-empty">노드 정보 없음: ${sel}</div>`;
      return;
    }

    // Phase 68: 인스펙터 영역 분리
    // 1) 헤더 — 아이콘 + 라벨 + 설정 가능 배지
    const hasSettings = this._pbNodeHasSettings(nodeDef);
    const badge = hasSettings
      ? '<span class="inspector-badge has">⚙ 설정 가능</span>'
      : '<span class="inspector-badge auto">자동 동작</span>';
    let html = `<div class="inspector-header">
      <h4>${nodeDef.icon || ''} ${nodeDef.label}</h4>
      ${badge}
    </div>`;

    // 2) 설명 영역 (회색 톤, 정보성)
    html += `<div class="inspector-block info-block">`;
    if (nodeDef.desc) html += `<div class="info-line">${nodeDef.desc}</div>`;
    if (nodeDef.why)  html += `<div class="info-row"><span class="info-tag">왜?</span><span class="info-text">${nodeDef.why}</span></div>`;
    if (nodeDef.how)  html += `<div class="info-row"><span class="info-tag">어떻게?</span><span class="info-text">${nodeDef.how}</span></div>`;
    html += `</div>`;

    // 3) 설정 영역 (시안 액센트 강조)
    let hasFields = false;
    let settingsHtml = '';

    // configSections (글로벌 설정 — 섹션 전체)
    const sections = nodeDef.configSections || [];
    if (sections.length > 0 && this.state.configMeta && this.state.config) {
      sections.forEach(sk => {
        const fields = this.state.configMeta[sk];
        if (!fields) return;
        Object.entries(fields).forEach(([field, info]) => {
          hasFields = true;
          const value = this._getNestedValue(this.state.config, sk, field);
          const input = this._renderField(sk, field, info, value);
          settingsHtml += `<div class="settings-field setting-row">
            <div class="settings-field-header"><label>${field}</label><span class="setting-scope">글로벌</span></div>
            <div class="field-help">${info.description}</div>
            ${input}
          </div>`;
        });
      });
    }
    // Phase 69: configFields (섹션 내 특정 필드만 골라서)
    const configFields = nodeDef.configFields || [];
    if (configFields.length > 0 && this.state.configMeta && this.state.config) {
      configFields.forEach(([sk, fieldName]) => {
        const fields = this.state.configMeta[sk];
        if (!fields || !fields[fieldName]) return;
        const info = fields[fieldName];
        hasFields = true;
        const value = this._getNestedValue(this.state.config, sk, fieldName);
        const input = this._renderField(sk, fieldName, info, value);
        settingsHtml += `<div class="settings-field setting-row">
          <div class="settings-field-header"><label>${fieldName}</label><span class="setting-scope">글로벌</span></div>
          <div class="field-help">${info.description}</div>
          ${input}
        </div>`;
      });
    }

    // 노드 자체 fields (스텝별 오버라이드)
    if (nodeDef.fields && nodeDef.fields.length) {
      const vals = (this.pb.nodeValues && this.pb.nodeValues[sel]) || {};
      nodeDef.fields.forEach(f => {
        if (f.showIf && !f.showIf(vals)) return;
        hasFields = true;
        const val = vals[f.key];
        settingsHtml += `<div class="settings-field setting-row">
          <div class="settings-field-header"><label>${f.label || f.key}</label><span class="setting-scope step">노드 옵션</span></div>`;
        if (f.desc) settingsHtml += `<div class="field-help">${f.desc}</div>`;
        if (f.type === 'credential') {
          const creds = this.state._cachedCredentials || [];
          const opts = ['<option value="">기본 (미지정)</option>']
            .concat(creds.map(c => `<option value="${c.id}" ${val === c.id ? 'selected' : ''}>${this._escHtml(c.label || c.provider)}</option>`));
          settingsHtml += `<select data-pb-node="${sel}" data-pb-field="${f.key}">${opts.join('')}</select>`;
        } else if (f.type === 'prompt_editor') {
          settingsHtml += `<button class="btn btn-secondary" data-action="edit-prompts" style="font-size:.78rem">프롬프트 편집</button>`;
        } else if (f.type === 'checkbox') {
          settingsHtml += `<input type="checkbox" data-pb-node="${sel}" data-pb-field="${f.key}" ${val ? 'checked' : ''}>`;
        } else if (f.type === 'number') {
          settingsHtml += `<input type="number" data-pb-node="${sel}" data-pb-field="${f.key}" value="${val ?? f.default ?? ''}">`;
        } else {
          settingsHtml += `<input type="text" data-pb-node="${sel}" data-pb-field="${f.key}" value="${this._escHtml(val ?? '')}">`;
        }
        settingsHtml += `</div>`;
      });
    }

    // Phase 92 H5: remote_upload 노드일 때 현재 활성 어댑터 capability 표시
    if (sel === 'remote_upload') {
      html += `<div class="inspector-block info-block" id="remote-storage-cap" style="border:1px dashed var(--color-border);margin-top:6px">
        <div class="info-line" style="font-weight:600">🗂 활성 백엔드 capability</div>
        <div class="info-line" style="font-size:.75rem;color:var(--color-text-dim)">불러오는 중...</div>
      </div>`;
      // 비동기 로드 후 채움
      setTimeout(() => { this._loadRemoteStorageCapInline(); }, 0);
    }

    if (hasFields) {
      html += `<div class="inspector-block settings-block">
        <div class="block-title">⚙ 설정</div>
        ${settingsHtml}
        <div class="inspector-footer">
          <span id="pb-inspector-saving" class="save-status"></span>
          <button class="btn btn-primary" data-action="pb-inspector-save">저장</button>
        </div>
      </div>`;
    } else {
      html += `<div class="inspector-block auto-block">
        <div class="block-title-muted">자동 동작</div>
        <div class="field-help">이 노드는 별도 설정 없이 자동으로 동작합니다.</div>
      </div>`;
    }
    host.innerHTML = html;
  },

  _renderSearchInspector(nodeId) {
    const meta = this.state.configMeta || {};
    const config = this.state.config || {};
    const descMap = {
      query_input:     ['쿼리 입력',       '사용자가 입력한 텍스트/모드/필터를 받는 진입점.', null],
      query_normalize: ['쿼리 정규화',     '공백·대소문자·이모지 정리. 동일 의미 쿼리를 동일 키로 매핑.', null],
      query_sensitive: ['민감 차단',       '민감 키워드 쿼리는 sensitive 인덱스에서 제외 (가공의 민감 판별과 정책 공유).', 'sensitive'],
      query_cache:     ['쿼리 캐시 조회', '동일 쿼리 SHA-256으로 최근 결과 재사용 (placeholder, 후속 구현 예정).', null],
      query_expand:    ['쿼리 확장',       '검색 모드 5종 분기 (default/exact/related/recent/fusion). 모드는 검색 호출 시 인자로 전달. HyDE는 트리거 #6 발동 시 활성.', 'search'],
      embed:           ['Embed',           'fastembed BGE-M3로 쿼리 → 1024차원 벡터 (인덱싱과 동일 모델, 고정).', null],
      fuse:            ['Fuse (RRF)',      'Reciprocal Rank Fusion + 메타데이터 필터 + 시간 가중. time_weight으로 recent boost 조정.', 'search'],
      rerank:          ['Rerank',          'BGE-Reranker-v2-M3 Cross-Encoder로 정밀 점수.', 'rerank'],
      win:             ['Sentence Window', '매칭 위치 ±N 줄 발췌.', 'search'],
      crag:            ['CRAG',            '신뢰도 3단계 + 보완 검색 (placeholder, 후속 구현).', null],
      parent:          ['Parent Expand',   '자식 청크 → 부모 섹션 교체 (트리거 #7 발동 시).', null],
      mmr:             ['MMR 다양성',      'Maximal Marginal Relevance로 결과 다양성 보장.', 'search'],
      hybrid:          ['Hybrid Match',    'Dense 코사인 + Sparse BM25 병렬 매칭. sparse_weight으로 비율 조정.', 'search'],
      // Phase 69: 가공 후처리와 미러
      kg_attach:       ['관련 문서 첨부', '검색 결과에 KG 이웃(교차참조 관계) 자동 첨부 (가공의 교차참조 미러).', null],
      entity_hl:       ['엔티티 하이라이트', '검색 결과 텍스트에서 추출된 엔티티(인명/금액/URL) 하이라이트 (가공의 엔티티 추출 미러).', null],
      paging:          ['결과 정렬·페이징', '점수순 정렬 + topK 페이징.', 'vector_db'],
      deliver:         ['MCP/UI 전달',     'MCP stdio 또는 Dashboard로 결과 반환.', null],
      log:             ['검색 로그 기록',  '[mcp-usage] 태그로 호출 기록.', null],
      notify:          ['알림',            '검색 실패 누적 임계 도달 시 HyDE 후보 알림.', null],
      metric:          ['품질 메트릭',     '골든셋 매칭 시 Recall@K, MRR 자동 갱신.', null],
    };
    const entry = descMap[nodeId];
    if (!entry) return `<div class="pb-inspector-empty">알 수 없는 노드: ${nodeId}</div>`;
    const [label, desc, configKey] = entry;
    const hasSettings = configKey && meta[configKey];
    const badge = hasSettings
      ? '<span class="inspector-badge has">⚙ 설정 가능</span>'
      : '<span class="inspector-badge auto">자동 동작</span>';
    let html = `<div class="inspector-header"><h4>${label}</h4>${badge}</div>`;
    html += `<div class="inspector-block info-block"><div class="info-line">${desc}</div></div>`;
    if (hasSettings) {
      let settingsHtml = '';
      Object.entries(meta[configKey]).forEach(([field, info]) => {
        const value = this._getNestedValue(config, configKey, field);
        const input = this._renderField(configKey, field, info, value);
        settingsHtml += `<div class="settings-field setting-row">
          <div class="settings-field-header"><label>${field}</label><span class="setting-scope">글로벌</span></div>
          <div class="field-help">${info.description}</div>
          ${input}
        </div>`;
      });
      html += `<div class="inspector-block settings-block">
        <div class="block-title">⚙ 설정</div>
        ${settingsHtml}
        <div class="inspector-footer">
          <span id="pb-inspector-saving" class="save-status"></span>
          <button class="btn btn-primary" data-action="pb-inspector-save">저장</button>
        </div>
      </div>`;
    } else {
      html += `<div class="inspector-block auto-block">
        <div class="block-title-muted">자동 동작</div>
        <div class="field-help">이 노드는 별도 설정 없이 자동으로 동작합니다.</div>
      </div>`;
    }
    return html;
  },

  // Phase 67: 인스펙터의 모든 [data-section] 입력을 수집해 config에 반영 + 저장
  async _saveInspectorChanges() {
    if (!this.state.config) return;
    const config = JSON.parse(JSON.stringify(this.state.config));
    const inspector = document.getElementById('pb-inspector');
    if (!inspector) return;
    inspector.querySelectorAll('[data-section]').forEach(el => {
      const section = el.dataset.section;
      const field = el.dataset.field;
      let value;
      if (el.type === 'checkbox') value = el.checked;
      else if (el.type === 'number') value = el.step === '1' ? parseInt(el.value) || 0 : parseFloat(el.value) || 0;
      else if (el.tagName === 'SELECT') value = isNaN(el.value) ? el.value : parseFloat(el.value);
      else if (el.dataset.array) {
        if (el.classList.contains('array-table')) {
          value = Array.from(el.querySelectorAll('.array-row input')).map(inp => inp.value.trim()).filter(Boolean);
        } else {
          value = el.value.split(',').map(s => s.trim()).filter(Boolean);
        }
      } else value = el.value;
      if (el.type === 'password' && value === '****') return;
      // 중첩 섹션 (verification.thresholds 등)
      const sectionParts = section.split('.');
      let obj = config;
      for (const p of sectionParts) {
        if (!obj[p]) obj[p] = {};
        obj = obj[p];
      }
      if (field.includes('.')) {
        const fp = field.split('.');
        let target = obj;
        for (let i = 0; i < fp.length - 1; i++) {
          if (!target[fp[i]]) target[fp[i]] = {};
          target = target[fp[i]];
        }
        target[fp[fp.length - 1]] = value || null;
      } else {
        obj[field] = value;
      }
    });
    // 노드별 옵션 (pb-node/pb-field) — credential, prompt 등
    inspector.querySelectorAll('[data-pb-node][data-pb-field]').forEach(el => {
      const nodeId = el.dataset.pbNode;
      const fkey = el.dataset.pbField;
      let v;
      if (el.type === 'checkbox') v = el.checked;
      else if (el.type === 'number') v = parseFloat(el.value) || 0;
      else v = el.value;
      if (!this.pb.nodeValues[nodeId]) this.pb.nodeValues[nodeId] = {};
      this.pb.nodeValues[nodeId][fkey] = v;
    });
    const status = document.getElementById('pb-inspector-saving');
    if (status) status.textContent = '저장 중...';
    try {
      const result = await API.updateConfig(config);
      if (result?.errors) {
        if (status) { status.textContent = result.errors.join(', '); status.style.color = 'var(--color-error)'; }
        return;
      }
      this.state.config = config;
      if (status) { status.textContent = '✓ 저장됨'; status.style.color = 'var(--color-success)'; }
      // 저장 후 노드 인스펙터 재렌더 (값 동기화)
      setTimeout(() => { if (status) status.textContent = ''; }, 2000);
    } catch(e) {
      if (status) { status.textContent = '저장 실패: ' + e; status.style.color = 'var(--color-error)'; }
    }
  },

  _renderBatchInspector(sectionKey) {
    const meta = this.state.configMeta || {};
    const fields = meta[sectionKey];
    const labels = { schedule: '스케줄', max_workers: '동시성', retention: '보존·Purge', preprocessing: '토픽 자동 병합' };
    if (!fields) return `<div class="pb-inspector-empty">${sectionKey} 섹션 메타가 없습니다.</div>`;
    const config = this.state.config || {};
    let html = `<div class="inspector-header">
      <h4>${labels[sectionKey] || sectionKey}</h4>
      <span class="inspector-badge has">⚙ 설정 가능</span>
    </div>`;
    let settingsHtml = '';
    Object.entries(fields).forEach(([field, info]) => {
      const value = this._getNestedValue(config, sectionKey, field);
      const input = this._renderField(sectionKey, field, info, value);
      settingsHtml += `<div class="settings-field setting-row">
        <div class="settings-field-header"><label>${field}</label><span class="setting-scope">글로벌</span></div>
        <div class="field-help">${info.description}</div>
        ${input}
      </div>`;
    });
    html += `<div class="inspector-block settings-block">
      <div class="block-title">⚙ 설정</div>
      ${settingsHtml}
      <div class="inspector-footer">
        <span id="pb-inspector-saving" class="save-status"></span>
        <button class="btn btn-primary" data-action="pb-inspector-save">저장</button>
      </div>
    </div>`;
    return html;
  },

  _initPBNodeValues() {
    const vals = {};
    Object.entries(this.PB_NODES).forEach(([id, def]) => {
      vals[id] = {};
      (def.fields || []).forEach(f => { if (f.default !== undefined) vals[id][f.key] = f.default; });
    });

    // config에서 글로벌 현재값 로드
    const cfg = this.state.config;
    if (cfg) {
      if (cfg.schedule?.fragment_threshold !== undefined) vals.fragment.fragment_threshold = cfg.schedule.fragment_threshold;
      // sensitive keywords/extensions from config (stored as arrays for tags)
      if (cfg.sensitive?.keywords) vals.sensitive.keywords = Array.isArray(cfg.sensitive.keywords) ? cfg.sensitive.keywords : String(cfg.sensitive.keywords).split('\n').filter(Boolean);
      if (cfg.sensitive?.extensions) vals.sensitive.extensions = Array.isArray(cfg.sensitive.extensions) ? cfg.sensitive.extensions : String(cfg.sensitive.extensions).split('\n').filter(Boolean);
      if (cfg.preprocessing?.pdf_tool) vals.preprocess.pdf_tool = cfg.preprocessing.pdf_tool;
      if (cfg.preprocessing?.ocr_tool) vals.preprocess.ocr_tool = cfg.preprocessing.ocr_tool;
      // Phase 65: 임베딩은 fastembed 고정. UI에서 model/onnx_model_dir 미노출.
      if (cfg.compression?.zstd_level !== undefined) vals.save_compress.zstd_level = cfg.compression.zstd_level;
      if (cfg.llm?.credential) vals.llm.credential = cfg.llm.credential;
      if (cfg.verification?.enabled !== undefined) vals.verify.enabled = cfg.verification.enabled;
      if (cfg.verification?.credential) vals.verify.credential = cfg.verification.credential;
    }

    vals.sensitive.keywords = vals.sensitive.keywords || [];
    vals.sensitive.extensions = vals.sensitive.extensions || [];

    this.pb.nodeValues = vals;
  },

  // ── 사이드바 렌더 (시뮬레이션 + 로그) ──
  renderPBSidebar() {
    const el = document.getElementById('pb-sidebar');
    if (!el) return;
    let html = '<div class="pb-sidebar-title">시뮬레이션</div>';
    html += '<div style="font-size:.72rem;color:var(--color-text-dim);margin-bottom:var(--spacing-sm)">실제 LLM 호출 + 검증을 실행합니다 (DB 저장 없음).</div>';
    html += `<div class="pb-sim-input">
      <textarea id="pb-sim-text" placeholder="테스트 텍스트를 입력하세요...">${this._escHtml(this.pb.simInput)}</textarea>
    </div>`;
    html += `<div class="pb-sidebar-actions" style="margin-top:4px">
      <button class="btn btn-primary" data-action="pb-sim-run" style="font-size:.75rem;padding:3px 8px">실행</button>
      ${this.pb.simResults ? '<button class="btn btn-secondary" data-action="pb-sim-clear" style="font-size:.75rem;padding:3px 8px">초기화</button>' : ''}
    </div>`;
    // Simulation results summary
    if (this.pb.simResults) {
      const nodes = this._pbCurrentNodes();
      html += '<div class="pb-sidebar-divider"></div>';
      html += '<div class="pb-sidebar-title">결과</div>';
      nodes.forEach(id => {
        const def = this.PB_NODES[id];
        const r = this.pb.simResults[id];
        if (!def) return;
        const dot = r ? `<span class="pb-sim-dot ${r.status}"></span>` : '<span class="pb-sim-dot skip"></span>';
        const label = r ? (r.status === 'pass' ? 'PASS' : r.status === 'skip' ? 'SKIP' : 'FAIL') : '-';
        html += `<div style="display:flex;align-items:center;gap:6px;padding:2px 0;font-size:.78rem;color:var(--color-text-muted)">
          ${dot} <span>${def.icon} ${def.label}</span>
          <span style="margin-left:auto;font-size:.7rem">${label}</span>
        </div>`;
      });
    }

    // 시뮬레이션 로그
    html += '<div class="pb-sidebar-divider"></div>';
    html += '<div class="pb-sidebar-title">로그</div>';
    html += `<div id="pb-sim-log" style="font-family:var(--font-mono);font-size:.7rem;color:var(--color-text-dim);max-height:200px;overflow-y:auto;background:var(--color-bg);border-radius:4px;padding:var(--spacing-sm)">${this.pb.simLog || '시뮬레이션을 실행하면 로그가 표시됩니다.'}</div>`;

    el.innerHTML = html;
  },

  _escHtml(s) { return String(s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); },
  _formatBytes(b) { if (!b) return '0 B'; const u=['B','KB','MB','GB']; let i=0; while(b>=1024&&i<u.length-1){b/=1024;i++;} return b.toFixed(i?1:0)+' '+u[i]; },

  // ── 캔버스 렌더 (축소 플로우) ──
  renderPBCanvas() {
    const el = document.getElementById('pb-canvas');
    if (!el) return;

    // Phase 71: 24노드 (quarantine 분기 + memory_tier + lint 신규)
    const groupNodes = {
      precheck: ['input_source','sensitive','fragment','sha256','incremental'],
      step: ['preprocess','chunking','llm','verify'],
      post: ['quarantine','embed_gen','semantic_dup','save_compress','remote_upload','vectordb','crossref','entity_extract','todo_merge','topic_merge','memory_tier','lint','notify','compile_state'],
    };

    let html = '';
    this.PB_GROUPS.forEach((grp, gi) => {
      html += `<div class="pb-group">
        <div class="pb-group-header">${grp.label}</div>`;
      html += '<div class="pb-nodes">';
      const nodes = groupNodes[grp.id] || [];
      nodes.forEach((id, i) => {
        if (i > 0) html += '<span class="pb-node-arrow">\u2192</span>';
        html += this._renderPBNodeCard(id);
      });
      html += '</div></div>';
      if (gi < this.PB_GROUPS.length - 1) html += '<div class="pb-connector">\u2193</div>';
    });

    el.innerHTML = html;
  },

  // Phase 68: 노드가 설정 가능한지 판정 (configSections 또는 fields 존재 여부)

  _pbNodeHasSettings(def) {
    if (!def) return false;
    const meta = this.state.configMeta || {};
    const hasConfig = (def.configSections || []).some(sk =>
      meta[sk] && Object.keys(meta[sk]).length > 0);
    // Phase 79: configFields(섹션 내 일부 필드)도 설정 가능 노드로 인식
    const hasConfigFields = (def.configFields || []).some(([sk, fk]) =>
      meta[sk] && meta[sk][fk]);
    const hasFields = (def.fields || []).length > 0;
    return hasConfig || hasConfigFields || hasFields;
  },

  _renderPBNodeCard(nodeId) {
    const def = this.PB_NODES[nodeId];
    if (!def) return '';
    const simR = this.pb.simResults?.[nodeId];
    const simDot = simR ? `<span class="pb-sim-dot ${simR.status}"></span>` : '';
    const sel = this.pb.selectedInspector === nodeId ? ' active' : '';
    // Phase 68: 설정 가능 마커
    const hasSettings = this._pbNodeHasSettings(def);
    const cogClass = hasSettings ? ' has-settings' : ' auto-only';
    const cogIcon = hasSettings ? '<span class="pb-node-cog" title="설정 가능">⚙</span>' : '';
    // 분기 노드 시각 마커: 점선 보더 + 분기 라벨
    const branchClass = def.branch_from ? ' pb-node-branch' : '';
    const branchTitle = def.branch_from
      ? ` title="분기: ${def.branch_from} → ${def.label} (조건: ${def.branch_condition || ''})"`
      : '';
    const branchBadge = def.branch_from
      ? `<span class="pb-node-branch-badge" title="${this._escHtml(def.branch_condition || '')}">↘ FAIL 분기</span>`
      : '';

    return `<div class="pb-node${sel}${cogClass}${branchClass}" data-action="pb-inspector-node" data-node-id="${nodeId}" data-node-label="${def.label}" style="cursor:pointer"${branchTitle}>
      <span class="pb-node-icon">${def.icon}</span>
      <span class="pb-node-label">${def.label}</span>
      ${branchBadge}
      ${cogIcon}
      ${simDot}
    </div>`;
  },


  // ── 공통: 필드 렌더 헬퍼 ──
  _renderPBField(nodeId, f, nodeVals) {
    const val = nodeVals[f.key] ?? f.default ?? '';
    let html = `<div class="pb-insp-field"><label>${f.label}</label>`;
    if (f.desc) html += `<div class="field-desc">${f.desc}</div>`;

    if (f.type === 'tags') {
      const tags = Array.isArray(val) ? val : (val ? String(val).split('\n').filter(Boolean) : []);
      html += `<div class="tag-input-container">
        <div class="tag-list">${tags.map(t => `<span class="tag">${this._escHtml(t)} <button data-action="pb-remove-tag" data-node="${nodeId}" data-key="${f.key}" data-val="${this._escHtml(t)}">×</button></span>`).join('')}</div>
        <input type="text" placeholder="입력 후 Enter..." class="tag-add-input" data-pb-step="${nodeId}" data-pb-key="${f.key}">
      </div>`;
    } else if (f.type === 'select') {
      html += `<select data-pb-step="${nodeId}" data-pb-key="${f.key}">${(f.options||[]).map(o=>`<option value="${o}"${o===String(val)?' selected':''}>${o}</option>`).join('')}</select>`;
      if (f.testable) {
        const toolMap = { 'marker':'marker','pymupdf4llm':'pymupdf4llm','tesseract':'tesseract','claude_vision':'claude_vision','pandoc':'pandoc','python':String(val).includes('docx')?'python-docx':'openpyxl','libreoffice':'libreoffice' };
        const testTool = val === 'auto' ? f.testable : (toolMap[val] || val);
        if (val !== 'none') {
          html += `<button data-action="test-tool" data-tool="${testTool}" style="margin-left:var(--spacing-xs);font-size:.7rem;padding:2px 6px;cursor:pointer;background:var(--color-surface);border:1px solid var(--color-border);color:var(--color-text-muted);border-radius:4px">\ud83d\udd0d 테스트</button>`;
          html += `<span id="tool-test-${f.key}" style="font-size:.7rem;margin-left:4px"></span>`;
        }
      }
    } else if (f.type === 'checkbox') {
      html += `<input type="checkbox" data-pb-step="${nodeId}" data-pb-key="${f.key}" ${val?'checked':''}>`;
    } else if (f.type === 'range') {
      html += `<div class="pb-range-row"><input type="range" min="${f.min}" max="${f.max}" step="${f.step}" value="${val}" data-pb-step="${nodeId}" data-pb-key="${f.key}"><span class="pb-range-val" id="pb-rv-${nodeId}-${f.key}">${val}</span></div>`;
      if (f.key === 'zstd_level') {
        const lvl = parseInt(val) || 3;
        const desc = lvl <= 1 ? '최고 속도, 최소 압축' : lvl <= 3 ? '��본값 (속도\u2194압축 균형)' : lvl <= 6 ? '약간 느림, 좋은 압축' : lvl <= 9 ? '느림, 높은 압축' : lvl <= 15 ? '매우 느림, 최대급 압축' : lvl <= 19 ? '극한 압축 (>10초/파일)' : '최대 압축 (매우 느림)';
        html += `<div style="font-size:.72rem;color:var(--color-text-dim);margin-top:2px">${desc}</div>`;
      }
    } else if (f.type === 'credential') {
      const creds = this.state._cachedCredentials || [];
      const defaultCredName = this.state.config?.llm?.default_credential;
      const defaultCred = defaultCredName ? creds.find(c => c.name === defaultCredName) : null;
      const defaultLabel = defaultCred ? `기본 — ${defaultCred.name} (${defaultCred.provider})` : '기본 (미지정)';
      html += `<select data-pb-step="${nodeId}" data-pb-key="${f.key}"><option value=""${!val?' selected':''}>${defaultLabel}</option>${creds.map(c=>`<option value="${c.name}"${c.name===val?' selected':''}>${c.name} (${c.provider})</option>`).join('')}</select>`;
    } else if (f.type === 'prompt_editor') {
      html += `<button data-action="pb-edit-prompts" class="btn btn-secondary" style="font-size:.78rem;padding:4px 10px">프롬프트 편집</button>`;
    } else {
      html += `<input type="${f.type||'text'}" value="${this._escHtml(val)}" data-pb-step="${nodeId}" data-pb-key="${f.key}">`;
    }
    html += '</div>';
    return html;
  },

  // ── 공���: 필드 이벤트 바인딩 ──
  _bindPBFields(container) {
    container.querySelectorAll('[data-pb-step]').forEach(input => {
      const handler = () => {
        const nodeId = input.dataset.pbStep;
        const key = input.dataset.pbKey;
        if (!this.pb.nodeValues[nodeId]) this.pb.nodeValues[nodeId] = {};
        if (input.type === 'checkbox') this.pb.nodeValues[nodeId][key] = input.checked;
        else if (input.type === 'number' || input.type === 'range') this.pb.nodeValues[nodeId][key] = parseFloat(input.value) || 0;
        else this.pb.nodeValues[nodeId][key] = input.value;
        // Update range display
        const rv = document.getElementById(`pb-rv-${nodeId}-${key}`);
        if (rv) rv.textContent = input.value;
        this._pbAutoSave();
      };
      input.addEventListener('change', handler);
      if (input.type === 'text' || input.type === 'number') input.addEventListener('input', handler);
    });
    // Tag add inputs
    container.querySelectorAll('.tag-add-input').forEach(input => {
      input.addEventListener('keydown', (e) => {
        if (e.key !== 'Enter') return;
        e.preventDefault();
        const val = input.value.trim();
        if (!val) return;
        const nodeId = input.dataset.pbStep;
        const key = input.dataset.pbKey;
        if (!this.pb.nodeValues[nodeId]) this.pb.nodeValues[nodeId] = {};
        if (!Array.isArray(this.pb.nodeValues[nodeId][key])) this.pb.nodeValues[nodeId][key] = [];
        this.pb.nodeValues[nodeId][key].push(val);
        input.value = '';
        this._pbAutoSave();
      });
    });
  },

  _bindSystemFields(container, cfg) {
    container.querySelectorAll('[data-pb-system]').forEach(input => {
      const handler = () => {
        const path = input.dataset.pbSystem.split('.');
        let obj = cfg;
        for (let i = 0; i < path.length - 1; i++) { if (!obj[path[i]]) obj[path[i]] = {}; obj = obj[path[i]]; }
        const key = path[path.length - 1];
        if (input.type === 'checkbox') obj[key] = input.checked;
        else if (input.type === 'number') obj[key] = parseFloat(input.value) || 0;
        else obj[key] = input.value;
        this._pbAutoSave();
      };
      input.addEventListener('change', handler);
      if (input.type === 'text' || input.type === 'password' || input.type === 'number') input.addEventListener('input', handler);
    });
  },


  // ── 이벤트 핸들러 ──
  async handlePBAction(action, el, e) {
    switch(action) {
      case 'pb-sim-run': {
        this.pb.simInput = document.getElementById('pb-sim-text')?.value || '';
        this._pbSimulate();
        break;
      }
      case 'pb-sim-clear': {
        this.pb.simResults = null;
        this.pb.simLog = '';
        this.renderPBCanvas(); this.renderPBSidebar();
        break;
      }
      case 'pb-remove-tag': {
        const node = el.dataset.node;
        const key = el.dataset.key;
        const val = el.dataset.val;
        if (node && key && vm.pb.nodeValues?.[node]) {
          const arr = vm.pb.nodeValues[node][key];
          if (Array.isArray(arr)) {
            vm.pb.nodeValues[node][key] = arr.filter(t => t !== val);
            vm._pbAutoSave();
          }
        }
        break;
      }
      case 'pb-edit-prompts': {
        this._openPromptEditor();
        break;
      }
    }
  },

  _openHookModal(state) {
    // state = null (추가) | { index, hook } (편집)
    const isEdit = !!state;
    const h = state?.hook || { event: 'process_complete', webhook_url: null, command: null, enabled: true };
    const title = isEdit ? `이벤트 훅 편집 — #${state.index + 1}` : '이벤트 훅 추가';
    const targetType = h.webhook_url ? 'webhook' : 'command';
    const target = h.webhook_url || h.command || '';
    const events = [
      ['file_detected', 'file_detected — inbox 파일 감지'],
      ['process_start', 'process_start — 가공 시작'],
      ['process_complete', 'process_complete — 가공 완료'],
      ['verify_fail', 'verify_fail — 검증 실패'],
      ['search_query', 'search_query — 검색 쿼리 수신'],
    ];
    const eventOpts = events.map(([v, l]) => `<option value="${v}"${v === h.event ? ' selected' : ''}>${l}</option>`).join('');
    const bodyHtml = `
      <div class="form-group">
        <label>이벤트</label>
        <select id="hook-event">${eventOpts}</select>
      </div>
      <div class="form-group">
        <label>대상 유형</label>
        <div style="display:flex;gap:var(--spacing-md);font-size:.85rem">
          <label><input type="radio" name="hook-target-type" value="webhook" ${targetType === 'webhook' ? 'checked' : ''}> HTTP 웹훅</label>
          <label><input type="radio" name="hook-target-type" value="command" ${targetType === 'command' ? 'checked' : ''}> 쉘 명령</label>
        </div>
      </div>
      <div class="form-group">
        <label id="hook-target-label">${targetType === 'webhook' ? 'Webhook URL' : '명령'}</label>
        <input type="text" id="hook-target" value="${this._escHtml(target)}" placeholder="${targetType === 'webhook' ? 'https://hooks.example.com/...' : 'notify-send "가공 완료"'}">
        <div class="form-help">이벤트 발생 시 payload(JSON)가 POST되거나 명령이 실행됩니다 (fire-and-forget)</div>
      </div>
      <div class="form-group">
        <label><input type="checkbox" id="hook-enabled" ${h.enabled ? 'checked' : ''}> 활성화</label>
      </div>`;

    Modal.open(title, bodyHtml, {
      onSave: async (overlay) => {
        const event = overlay.querySelector('#hook-event')?.value;
        const type = overlay.querySelector('input[name="hook-target-type"]:checked')?.value || 'webhook';
        const tgt = overlay.querySelector('#hook-target')?.value?.trim();
        const enabled = overlay.querySelector('#hook-enabled')?.checked || false;
        if (!event) throw new Error('이벤트를 선택하세요');
        if (!tgt) throw new Error('대상(URL 또는 명령)을 입력하세요');
        const newHook = {
          event,
          webhook_url: type === 'webhook' ? tgt : null,
          command: type === 'command' ? tgt : null,
          enabled,
        };
        const hooks = (vm.state.config.hooks || []).slice();
        if (isEdit) hooks[state.index] = newHook;
        else hooks.push(newHook);
        vm.state.config.hooks = hooks;
        const result = await API.updateConfig(vm.state.config);
        if (result?.error) throw new Error(result.error);
        await vm.loadSettings();
      }
    });
    // 라디오 변경 시 라벨/플레이스홀더 갱신
    setTimeout(() => {
      const overlay = document.querySelector('.modal-overlay');
      if (!overlay) return;
      overlay.querySelectorAll('input[name="hook-target-type"]').forEach(r => {
        r.addEventListener('change', (ev) => {
          const t = ev.target.value;
          const lbl = overlay.querySelector('#hook-target-label');
          const inp = overlay.querySelector('#hook-target');
          if (lbl) lbl.textContent = t === 'webhook' ? 'Webhook URL' : '명령';
          if (inp) inp.placeholder = t === 'webhook' ? 'https://hooks.example.com/...' : 'notify-send "가공 완료"';
        });
      });
    }, 0);
  },

  async _openPromptEditor() {
    const data = await API.getPrompts();
    const content = data?.content || '';

    const bodyHtml = `
      <div class="form-group">
        <label>prompts.toml — 저장 시 즉시 반영 (핫 리로드)</label>
        <textarea id="pb-prompt-textarea" style="min-height:400px;font-family:var(--font-mono);font-size:.8rem;resize:vertical;tab-size:2">${vm._escHtml(content)}</textarea>
      </div>
      <div style="font-size:.72rem;color:var(--color-text-dim)">변수: {type_hints}, {filename}, {content}, {feedback}, {existing}, {new_content}</div>`;

    Modal.open('프롬프트 편집', bodyHtml, {
      wide: true,
      saveLabel: '저장 + 적용',
      async onSave(overlay) {
        const textarea = overlay.querySelector('#pb-prompt-textarea');
        if (!textarea) throw new Error('textarea를 찾을 수 없습니다');
        const result = await API.savePrompts(textarea.value);
        if (!result?.ok) throw new Error(result?.error || '저장 실패');
      }
    });
  },

  // Auto-save: debounced save on field changes
  _pbAutoSaveTimer: null,
  _pbAutoSave() {
    if (this._pbAutoSaveTimer) clearTimeout(this._pbAutoSaveTimer);
    this._pbAutoSaveTimer = setTimeout(() => this._pbSave(), 1000);
  },

  async _pbSave() {
    const vals = this.pb.nodeValues || {};
    const cfg = this.state.config;
    if (!cfg) return;

    // Map node values back to config
    if (vals.sensitive) {
      if (!cfg.sensitive) cfg.sensitive = {};
      cfg.sensitive.keywords = Array.isArray(vals.sensitive.keywords) ? vals.sensitive.keywords : [];
      cfg.sensitive.extensions = Array.isArray(vals.sensitive.extensions) ? vals.sensitive.extensions : [];
    }
    if (vals.fragment?.fragment_threshold !== undefined) {
      if (!cfg.schedule) cfg.schedule = {};
      cfg.schedule.fragment_threshold = parseInt(vals.fragment.fragment_threshold) || 100;
    }
    if (vals.preprocess) {
      if (!cfg.preprocessing) cfg.preprocessing = {};
      cfg.preprocessing.pdf_tool = vals.preprocess.pdf_tool || 'none';
      cfg.preprocessing.ocr_tool = vals.preprocess.ocr_tool || 'none';
      cfg.preprocessing.docx_tool = vals.preprocess.docx_tool || 'auto';
      cfg.preprocessing.xlsx_tool = vals.preprocess.xlsx_tool || 'auto';
      cfg.preprocessing.pptx_tool = vals.preprocess.pptx_tool || 'auto';
    }
    if (vals.llm?.credential) {
      if (!cfg.llm) cfg.llm = {};
      cfg.llm.credential = vals.llm.credential;
    }
    if (vals.verify) {
      if (!cfg.verification) cfg.verification = {};
      cfg.verification.enabled = vals.verify.enabled !== false;
      if (vals.verify.credential) cfg.verification.credential = vals.verify.credential;
    }
    // Phase 65: 임베딩은 fastembed 고정. UI에서 저장 항목 없음.
    if (vals.save_compress?.zstd_level !== undefined) {
      if (!cfg.compression) cfg.compression = {};
      cfg.compression.zstd_level = parseInt(vals.save_compress.zstd_level) || 3;
    }

    const result = await API.updateConfig(cfg);
    if (result?.errors) { this._showMsg(result.errors.join(', '), 'error'); return; }
  },

  async _pbSimulate() {
    // 실제 dry-run: LLM 호출 + 검증 + 임베딩 실행. DB 저장/알림은 스킵.
    const input = this.pb.simInput || '';
    if (!input.trim()) { this._showMsg('테스트 텍스트를 입력하세요', 'error'); return; }

    this._showMsg('⏳ 시뮬레이션 실행 중 (LLM 호출 포함)...', 'info');

    try {
      const data = await API.simulatePipeline(input.trim());
      if (!data.steps) {
        this._showMsg('시뮬레이션 응답 오류', 'error');
        return;
      }

      // API 응답 → 노드별 결과 매핑
      const results = {};
      const nodeNames = this._pbCurrentNodes();
      const stepMap = {
        '민감 판별': 'sensitive',
        'Fragment 감지': 'fragment',
        'LLM 분류+가공': 'llm',
        '검증': 'verify',
        '임베딩': 'embed_gen',
        '임베딩 (Fragment)': 'embed_gen',
        '저장+압축': 'save_compress',
        '벡터DB 색인': 'vectordb',
        '교차참조': 'crossref',
        '알림': 'notify',
        '이후 스텝': null,
        '저장/색인': 'save_compress',
      };

      // 모든 노드 기본값
      nodeNames.forEach(n => {
        results[n] = { status: 'pass', input: '', output: '' };
      });

      // API 결과 매핑
      for (const step of data.steps) {
        const nodeId = stepMap[step.name];
        if (nodeId && results[nodeId]) {
          results[nodeId] = {
            status: step.status,
            input: step.name + (step.elapsed_ms ? ` (${step.elapsed_ms}ms)` : ''),
            output: step.output || '',
          };
        }
        // 민감 파일 → 이후 전체 스킵
        if (step.name === '이후 스텝' && step.status === 'skip') {
          nodeNames.forEach(n => {
            if (!results[n].output) results[n] = { status: 'skip', input: '—', output: '민감 파일 → 중단' };
          });
        }
      }

      // SHA-256/증분은 dry-run에서 판정 불가
      results.sha256 = { status: 'pass', input: 'SHA-256', output: '[dry-run] 스킵' };
      results.incremental = { status: 'pass', input: '증분 해시', output: '[dry-run] 스킵' };
      results.preprocess = results.preprocess || { status: 'pass', input: '전처리', output: '텍스트 입력' };

      this.pb.simResults = results;
      // 시뮬레이션 로그 생성
      let log = '';
      for (const step of data.steps) {
        const icon = step.status === 'pass' ? '\u2705' : step.status === 'fail' ? '\u274c' : '\u23ed';
        log += `${icon} ${step.name}`;
        if (step.elapsed_ms) log += ` (${step.elapsed_ms}ms)`;
        if (step.output) log += ` — ${step.output.substring(0,80)}`;
        log += '\n';
      }
      this.pb.simLog = log;
      this._showMsg(`\u2705 시뮬레이션 완료 (${data.total_ms}ms) — DB 저장 없음`, 'success');
    } catch (e) {
      this.pb.simLog = '\u274c 오류: ' + e;
      this._showMsg('시뮬레이션 실패: ' + e, 'error');
    }

    this.renderPBCanvas(); this.renderPBSidebar();
  },
};

// ── View 바인딩 ───────────────────────────────────────────────
function renderMigrationPanel() {
  return `<div class="settings-section" style="margin-top:var(--spacing-md)">
  <div class="migration-card">
    <h4>임베딩 모델 / 벡터 차원 변경</h4>
    <p class="migration-desc">임베딩 모델(Claude CLI/OpenAI/ONNX)이나 벡터 차원(dim)을 변경했을 때 기존 문서의 임베딩을 재생성합니다.</p>
    <div class="migration-warning">
      재수행 대상:
      <ul>
        <li><code>*.vec</code> — 임베딩 벡터 재생성</li>
        <li>LocalVectorStore — 재색인</li>
      </ul>
      <span style="color:var(--color-success)">유지: <code>*.zst</code> (가공본/원본) — 재가공 불필요</span>
    </div>
    <div class="migration-actions">
      <button class="btn btn-primary" data-action="rebuild-embeddings">임베딩 재생성 + 재색인</button>
      <button class="btn btn-secondary" data-action="rebuild-vectordb">벡터DB만 재구축</button>
    </div>
  </div>
  <div class="migration-card" style="margin-top:var(--spacing-md)">
    <h4>LLM 프로바이더 변경</h4>
    <p class="migration-desc">LLM(Claude/OpenAI/Ollama/Gemini)을 변경했을 때 모든 문서를 원본에서 다시 가공합니다.</p>
    <div class="migration-danger">
      전체 재가공 (시간 소요):
      <ul>
        <li><code>processed/*.zst</code> — 가공본 전체 재생성</li>
        <li><code>*.vec</code> + LocalVectorStore — 재색인</li>
      </ul>
      <span style="color:var(--color-text-dim)"><code>originals/*.zst</code>에서 복원 후 재가공합니다.</span>
    </div>
    <div class="migration-actions">
      <button class="btn btn-primary" style="background:var(--color-error)" data-action="rebuild-all">전체 재가공</button>
    </div>
  </div>
  <div id="migration-status" style="display:none;margin-top:var(--spacing-md)">
    <div class="migration-progress">
      <span id="migration-status-text">처리 중...</span>
      <div class="fb-progress"><div class="fb-progress-bar processing" id="migration-progress-bar" style="width:0%"></div></div>
    </div>
  </div>
</div>`;
}

document.addEventListener('DOMContentLoaded', () => {
  // 이벤트 위임
  document.addEventListener('click', async (e) => {
    const el = e.target.closest('[data-action]');
    if (!el) return;
    const action = el.dataset.action;
    if (action === 'show-doc') vm.showDoc(el.dataset.id);
    if (action === 'close-detail') { vm.state.currentDoc = null; vm.renderDocDetail(); }
    if (action === 'run-lint-strong-claims') vm.runLintStrongClaims();
    if (action === 'switch-tab') vm.switchTab(el.dataset.tab);
    if (action === 'pb-mid-tab') vm.switchPipelineMidTab(el.dataset.midtab);
    if (action === 'pb-inspector-node') {
      vm.pb.selectedInspector = el.dataset.nodeId;
      vm._renderPBInspector();
      // Active 표시 위해 가공/검색 영역 재렌더
      if (vm.pb.activeMidTab === 'search') vm._renderPBSearchPipeline();
      if (vm.pb.activeMidTab === 'process' && typeof vm.renderPBCanvas === 'function') vm.renderPBCanvas();
    }
    if (action === 'pb-batch-section') {
      vm.pb.selectedInspector = 'batch:' + el.dataset.section;
      vm._renderPBInspector();
    }
    if (action === 'pb-inspector-save') {
      vm._saveInspectorChanges();
    }
    if (action === 'edit-prompts') {
      if (typeof vm.openPromptsEditor === 'function') vm.openPromptsEditor();
    }
    if (action === 'open-onboarding') vm.startOnboarding();
    if (action === 'onboard-open-cred-form') vm._onboardOpenCredForm();
    // Phase 74/75: 설정 도우미
    if (action === 'open-setup-assistant') vm.openSetupAssistant();
    if (action === 'open-setup-fallback')  vm.openSetupFallback();
    if (action === 'setup-scenario-pick') vm._runSetupReview(el.dataset.scenario);
    if (action === 'setup-profile-review') vm._runSetupProfileReview();
    if (action === 'setup-preset-pick') vm._applySetupPreset(el.dataset.preset);
    if (action === 'setup-apply-submit') vm._runSetupApply(el.dataset.scenario);
    // Phase 80-1
    if (action === 'setup-quickstart-general') vm.setupQuickstartGeneral();
    if (action === 'open-setup-modules') vm.openSetupModules();
    if (action === 'setup-mcp-flow') {
      // 그대로 현재 모달 안의 MCP 안내 영역으로 — 다음 단계에서 보이게
      const body = document.querySelector('.modal-body');
      if (body) body.scrollTo({ top: body.scrollHeight / 2, behavior: 'smooth' });
    }
    if (action === 'modules-apply') vm._runModulesAction(false);
    if (action === 'modules-dryrun') vm._runModulesAction(true);
    if (action === 'search') vm.doSearch();
    if (action === 'toggle-watcher') vm._toggleWatcher();
    if (action === 'toggle-header') {
      const hg = document.getElementById('header-groups');
      const arrow = document.getElementById('header-arrow');
      if (hg) { hg.classList.toggle('collapsed'); if (arrow) arrow.textContent = hg.classList.contains('collapsed') ? '\u25b6' : '\u25bc'; }
    }
    if (action === 'run-auto-suggest') {
      try {
        const r = await API.autoSuggestFromCounters();
        vm._showMsg(`자동 추천 ${r.inserted}건 INSERT`, r.inserted > 0 ? 'success' : 'info');
        await vm.loadDecisionLog();
      } catch (e) {
        vm._showMsg(`자동 추천 실패: ${e}`, 'error');
      }
    }
    if (action === 'refresh-decision-log') {
      await vm.loadDecisionLog();
    }
    if (action === 'dl-filter-change') {
      await vm.loadDecisionLog();
    }
    if (action === 'accept-suggested') {
      const id = parseInt(el.dataset.id, 10);
      if (!Number.isFinite(id)) return;
      if (!confirm(`Decision Log #${id}을 pipeline.toml에 적용합니다. .bak 백업 후 진행됩니다.\n\n계속할까요?`)) return;
      try {
        const r = await API.acceptSuggestedDecision(id);
        vm._showMsg(`적용 완료: ${r.path} = ${r.after_value}`, 'success');
        await vm.loadDecisionLog();
      } catch (e) {
        vm._showMsg(`적용 실패: ${e}`, 'error');
      }
    }
    if (action === 'reject-suggested') {
      const id = parseInt(el.dataset.id, 10);
      if (!Number.isFinite(id)) return;
      try {
        await API.rejectSuggestedDecision(id);
        vm._showMsg(`Decision Log #${id} 거부 처리`, 'info');
        await vm.loadDecisionLog();
      } catch (e) {
        vm._showMsg(`거부 처리 실패: ${e}`, 'error');
      }
    }
    if (action === 'rollback-snapshot') {
      const snapshotId = el.dataset.snapshotId;
      if (!snapshotId) return;
      const reason = prompt('롤백 사유를 입력하세요 (예: "효과 측정 결과 회귀")', '사용자 수동 롤백');
      if (reason === null) return;
      try {
        await API.setupSnapshotRollback(snapshotId, reason || '사용자 수동 롤백');
        vm._showMsg(`스냅샷 ${snapshotId} 롤백 완료. pipeline.toml이 .bak 시점으로 복원됨`, 'success');
        await vm.loadDecisionLog();
      } catch (e) {
        vm._showMsg(`롤백 실패: ${e}`, 'error');
      }
    }
    if (action === 'add-hook') {
      vm._openHookModal(null);
    }
    if (action === 'edit-hook') {
      const idx = parseInt(el.dataset.index, 10);
      const hooks = vm.state.config?.hooks || [];
      if (idx >= 0 && idx < hooks.length) vm._openHookModal({ index: idx, hook: hooks[idx] });
    }
    if (action === 'delete-hook') {
      const idx = parseInt(el.dataset.index, 10);
      const hooks = (vm.state.config?.hooks || []).slice();
      if (idx < 0 || idx >= hooks.length) return;
      const h = hooks[idx];
      if (!confirm(`훅 "${h.event}"을 삭제할까요?`)) return;
      hooks.splice(idx, 1);
      vm.state.config.hooks = hooks;
      try {
        await API.updateConfig(vm.state.config);
        vm._showMsg('훅 삭제 완료', 'success');
        await vm.loadSettings();
      } catch (e) {
        vm._showMsg(`삭제 실패: ${e}`, 'error');
      }
    }
    if (action === 'refresh-c1-thresholds') {
      await vm.loadC1Thresholds();
    }
    if (action === 'c1-save') {
      const key = el.dataset.key;
      const input = document.querySelector(`input[data-c1-key="${key}"]`);
      if (!input) return;
      const raw = (input.value || '').trim();
      if (!raw) {
        vm._showMsg('값을 입력하세요 (비우면 디폴트 fallback)', 'info');
        return;
      }
      const value = parseFloat(raw);
      if (!Number.isFinite(value)) {
        vm._showMsg('숫자 값만 허용', 'error');
        return;
      }
      try {
        await API.c1ThresholdSet(key, value);
        vm._showMsg(`${key} = ${value} 저장`, 'success');
        await vm.loadC1Thresholds();
      } catch (e) {
        vm._showMsg(`저장 실패: ${e}`, 'error');
      }
    }
    if (action === 'refresh-pii-patterns') {
      await vm.loadPiiPatterns();
    }
    // Phase 93 GUI 가시화
    if (action === 'pii-mask-toggle') {
      await vm.togglePiiMask();
    }
    if (action === 'refresh-mcp-catalog') {
      await vm.loadMcpCatalog();
    }
    if (action === 'refresh-anomaly-report') {
      await vm.loadAnomalyReport();
    }
    if (action === 'pii-add') {
      const nameEl = document.getElementById('pii-new-name');
      const patEl = document.getElementById('pii-new-pattern');
      const name = (nameEl?.value || '').trim();
      const pattern = (patEl?.value || '').trim();
      if (!name || !pattern) {
        vm._showMsg('이름과 정규식 모두 입력 필수', 'error');
        return;
      }
      try {
        const r = await API.piiPatternAdd(name, pattern, true);
        const msg = r?.live_reloaded ? `PII 패턴 추가: ${name} (즉시 반영, 활성 ${r.active_count}건)` : `PII 패턴 추가: ${name}`;
        vm._showMsg(msg, 'success');
        if (nameEl) nameEl.value = '';
        if (patEl) patEl.value = '';
        await vm.loadPiiPatterns();
      } catch (e) {
        vm._showMsg(`PII 패턴 추가 실패 (regex 컴파일 확인): ${e}`, 'error');
      }
    }
    if (action === 'pii-remove') {
      const name = el.dataset.name;
      if (!name) return;
      if (!confirm(`PII 패턴 "${name}" 제거?`)) return;
      try {
        const r = await API.piiPatternRemove(name);
        const msg = r?.live_reloaded ? `PII 패턴 제거: ${name} (즉시 반영, 활성 ${r.active_count}건)` : `PII 패턴 제거: ${name}`;
        vm._showMsg(msg, 'success');
        await vm.loadPiiPatterns();
      } catch (e) {
        vm._showMsg(`PII 패턴 제거 실패: ${e}`, 'error');
      }
    }
    if (action === 'gc-llm-cache-now') {
      try {
        const r = await API.gcLlmCacheNow(null);
        vm._showMsg(`LRU GC 완료 — ${r.deleted}건 삭제 (max=${r.max_entries})`, 'success');
        vm.state._llmCacheStats = await API.getLlmCacheStats().catch(() => ({}));
        vm.renderStats();
      } catch (e) {
        vm._showMsg(`GC 실패: ${e}`, 'error');
      }
    }
    if (action === 'clear-llm-cache') {
      if (!confirm('LLM \uacb0\uacfc \uce90\uc2dc\ub97c \uc804\uccb4 \uc0ad\uc81c\ud569\ub2c8\ub2e4. \ubaa8\ub378/\ud504\ub86c\ud504\ud2b8 \ubcc0\uacbd \ud6c4 \uc0ac\uc6a9\ud569\ub2c8\ub2e4.\n\n\uacc4\uc18d\ud560\uae4c\uc694?')) return;
      try {
        const r = await API.clearLlmCache();
        vm._showMsg(`LLM \uce90\uc2dc \ube44\uc6c0: ${r.deleted}\uac74 \uc0ad\uc81c`, 'success');
        vm.state._llmCacheStats = await API.getLlmCacheStats().catch(() => ({}));
        vm.renderStats();
      } catch (e) {
        vm._showMsg(`\uce90\uc2dc \ube44\uc6b0\uae30 \uc2e4\ud328: ${e}`, 'error');
      }
    }
    if (action === 'save-settings') vm.saveSettings();
    if (action === 'reset-settings') { if (confirm('모든 변경사항을 되돌리시겠습니까?')) vm.loadSettings(); }
    if (action === 'export-config') vm.exportConfig();
    if (action === 'import-config') vm.importConfig();
    if (action === 'toggle-section') el.classList.toggle('collapsed');
    if (action === 'toggle-todo') vm.toggleTodo(el.dataset.id);
    if (action === 'add-todo') vm.openAddTodoModal();
    if (action === 'refresh-todos') { vm.state.todos = null; vm.loadTodos(); }
    if (action === 'open-topic') vm.openTopic(el.dataset.path);
    if (action === 'refresh-topics') { vm.state.topics = []; vm.loadTopics(); }
    if (action === 'new-credential') vm.showCredentialForm();
    if (action === 'edit-credential') vm.editCredential(el.dataset.credId);
    if (action === 'delete-credential') vm.deleteCredentialByName(el.dataset.name);
    if (action === 'set-default-credential') vm.setDefaultCredential(el.dataset.name);
    if (action === 'settings-nav') vm.settingsScrollTo(el.dataset.section);
    // ── Pipeline Builder 이벤트 (pb-* 액션) ──
    if (action?.startsWith('pb-')) vm.handlePBAction(action, el, e);
    // ── Migration 이벤트 ──
    if (action === 'rebuild-embeddings') {
      if (!confirm('임베딩을 재생성하고 벡터DB를 재색인합니다.\n*.vec 파일이 덮어쓰기됩니다.\n\n계속할까요?')) return;
      const statusEl = document.getElementById('migration-status');
      const textEl = document.getElementById('migration-status-text');
      const barEl = document.getElementById('migration-progress-bar');
      if (statusEl) statusEl.style.display = 'block';
      if (textEl) textEl.textContent = '임베딩 재생성 중...';
      if (barEl) barEl.style.width = '30%';
      const result = await API.rebuildEmbeddings();
      if (barEl) barEl.style.width = '100%';
      if (textEl) textEl.textContent = result.message || '완료';
      vm._showMsg(result.message || '임베딩 재생성 완료', result.ok ? 'success' : 'error');
    }
    if (action === 'rebuild-vectordb') {
      if (!confirm('벡터DB를 재구축합니다.\n기존 인덱스가 초기화됩니다.\n\n계속할까요?')) return;
      const statusEl = document.getElementById('migration-status');
      const textEl = document.getElementById('migration-status-text');
      const barEl = document.getElementById('migration-progress-bar');
      if (statusEl) statusEl.style.display = 'block';
      if (textEl) textEl.textContent = '벡터DB 재구축 중...';
      if (barEl) barEl.style.width = '30%';
      const result = await API.rebuildVectordb();
      if (barEl) barEl.style.width = '100%';
      if (textEl) textEl.textContent = result.message || '완료';
      vm._showMsg(result.message || '벡터DB 재구축 완료', result.ok ? 'success' : 'error');
    }
    if (action === 'rebuild-all') {
      if (!confirm('🔴 전체 재가공: 원본에서 모든 문서를 다시 가공합니다.\nLLM 호출 비용이 발생하며, 문서 수에 따라 수 분~수 시간 소요됩니다.\n\n⚠️ TTL(retention_days)에 의해 자동 제거된 원본은 포함되지 않습니다.\n\n정말 계속할까요?')) return;
      const statusEl = document.getElementById('migration-status');
      const textEl = document.getElementById('migration-status-text');
      const barEl = document.getElementById('migration-progress-bar');
      if (statusEl) statusEl.style.display = 'block';
      if (textEl) textEl.textContent = '원본을 inbox로 복사 중...';
      if (barEl) barEl.style.width = '10%';
      const result = await API.rebuildAll();
      if (barEl) barEl.style.width = '50%';
      if (textEl) textEl.textContent = result.message + ' 배치 처리는 Processing 탭에서 확인하세요.';
      vm._showMsg(result.message || '원본 복사 완료. 배치 처리를 시작합니다.', result.ok ? 'success' : 'error');
    }
    if (action === 'test-tool') {
      const tool = el.dataset.tool;
      const resultEl = el.nextElementSibling;
      if (resultEl) resultEl.textContent = '테스트 중...';
      const result = await API.testHostTool(tool);
      if (resultEl) {
        resultEl.textContent = result.ok ? `✅ ${result.version}` : `❌ ${result.error}`;
        resultEl.style.color = result.ok ? 'var(--color-success)' : 'var(--color-error)';
      }
    }
    if (action === 'retry-failed') vm.retryFailed();
    if (action === 'show-processing-log') vm.showProcessingLog(el.dataset.name);
    if (action === 'close-processing-log') vm.closeProcessingLog();
    if (action === 'proc-page') { vm.state._procPage = parseInt(el.dataset.page) || 1; vm._renderProcTable(); }
    if (action === 'prev-page') { vm.state.docPage = Math.max(1, vm.state.docPage - 1); vm.loadDocuments(); }
    if (action === 'next-page') { vm.state.docPage = Math.min(vm.state.docTotalPages, vm.state.docPage + 1); vm.loadDocuments(); }
  });

  // 검색 엔터키
  document.getElementById('search-query').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') vm.doSearch();
  });

  // 문서 유형 필터
  const filter = document.getElementById('doc-type-filter');
  if (filter) filter.addEventListener('change', () => vm.loadDocuments(filter.value || undefined));


  // Tag input: Enter key to add tag
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && e.target.classList.contains('tag-add-input')) {
      e.preventDefault();
      const val = e.target.value.trim();
      if (!val) return;
      const step = e.target.dataset.pbStep;
      const key = e.target.dataset.pbKey;
      if (!vm.pb.nodeValues[step]) vm.pb.nodeValues[step] = {};
      if (step && key && vm.pb?.nodeValues?.[step]) {
        if (!Array.isArray(vm.pb.nodeValues[step][key])) vm.pb.nodeValues[step][key] = [];
        if (!vm.pb.nodeValues[step][key].includes(val)) {
          vm.pb.nodeValues[step][key].push(val);
          e.target.value = '';
          vm._pbAutoSave();
        }
      }
    }
  });

  // 설정 검색
  const settingsSearch = document.getElementById('settings-search');
  if (settingsSearch) settingsSearch.addEventListener('input', () => vm.settingsFilterBySearch(settingsSearch.value));

  // Pipeline Builder 필드 change/input 위임
  document.addEventListener('change', (e) => {
    // Decision Log 필터 변경
    if (e.target.dataset.action === 'dl-filter-change') {
      vm.loadDecisionLog();
      return;
    }
    const step = e.target.dataset.pbStep;
    const key = e.target.dataset.pbKey;
    if (step && key && vm.pb?.nodeValues?.[step]) {
      vm.pb.nodeValues[step][key] = e.target.type === 'checkbox' ? e.target.checked : e.target.value;
      vm._pbAutoSave();
    }
  });
  document.addEventListener('input', (e) => {
    const step = e.target.dataset.pbStep;
    const key = e.target.dataset.pbKey;
    if (step && key && e.target.type === 'range') {
      const display = document.getElementById(`pb-rv-${step}-${key}`);
      if (display) display.textContent = e.target.value;
      if (vm.pb?.nodeValues?.[step]) vm.pb.nodeValues[step][key] = e.target.value;
      vm._pbAutoSave();
    }
  });

  // Resizable panels
  ['pb-divider-left','pb-divider-right'].forEach(id => {
    const div = document.getElementById(id);
    if (!div) return;
    let sx, sw;
    div.addEventListener('mousedown', e => {
      sx = e.clientX;
      const ly = document.getElementById('pb-layout');
      sw = getComputedStyle(ly).gridTemplateColumns;
      const onMove = ev => {
        const dx = ev.clientX - sx;
        const cols = sw.split(/\s+/).map(s => parseFloat(s) || 4);
        if (id === 'pb-divider-left') cols[0] = Math.max(160, cols[0]+dx);
        else cols[4] = Math.max(200, cols[4]-dx);
        ly.style.gridTemplateColumns = cols[0]+'px 4px 1fr 4px '+cols[4]+'px';
        sx = ev.clientX; sw = ly.style.gridTemplateColumns;
      };
      const onUp = () => { document.removeEventListener('mousemove',onMove); document.removeEventListener('mouseup',onUp); };
      document.addEventListener('mousemove',onMove);
      document.addEventListener('mouseup',onUp);
    });
  });

  // ── 우클릭 피드백 컨텍스트 메뉴 (소스 모드 전용) ──
  vm.init();
});
