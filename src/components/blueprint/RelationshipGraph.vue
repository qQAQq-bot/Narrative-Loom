<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import * as G6 from '@antv/g6';

const G6Any: any = (G6 as any).default ?? (G6 as any);

interface CharacterInfo {
  id: string;
  name: string;
  aliases: string[];
  role: string;
  relationships: unknown;
}

interface PairRelation {
  aId: string;
  bId: string;
  aToB: string[];
  bToA: string[];
}

interface UnresolvedRelation {
  sourceId: string;
  sourceName: string;
  targetRaw: string;
  relation: string;
  reason: 'not_found' | 'ambiguous';
  candidates?: string[];
}

const props = defineProps<{
  bookId: string;
  viewMode: 'timeline' | 'graph';
}>();

const emit = defineEmits<{
  (e: 'update:viewMode', value: 'timeline' | 'graph'): void;
}>();

const loading = ref(false);
const error = ref<string | null>(null);

const characters = ref<CharacterInfo[]>([]);

const graphContainerRef = ref<HTMLDivElement | null>(null);
let graph: any | null = null;
let resizeObserver: ResizeObserver | null = null;

const BASE_NODE_SIZE = 56;
const SELECTED_NODE_SIZE = 66;
let lastSelectedSizeId: string | null = null;

const graphNodes = ref<any[]>([]);
const graphEdges = ref<any[]>([]);
const pairRelations = ref<PairRelation[]>([]);
const unresolved = ref<UnresolvedRelation[]>([]);

const selectedId = ref<string | null>(null);

// Standalone fill color map — G6 cannot mutate this.
const nodeFillColors = new Map<string, string>();

// Track each node's visual role so hover can restore correctly.
type NodeRole = 'default' | 'selected' | 'neighbor' | 'dimmed';
const nodeVisualRoles = new Map<string, NodeRole>();

function getNodeStyle(id: string, role: NodeRole, hover = false) {
  const fill = nodeFillColors.get(id);
  const base = {
    default:  { lineWidth: 1,   stroke: 'rgba(255,255,255,0.9)',  shadowColor: 'rgba(0,0,0,0.10)',       shadowBlur: 14, opacity: 1    },
    selected: { lineWidth: 5,   stroke: '#ffffff',                shadowColor: 'rgba(139,92,246,0.35)',  shadowBlur: 26, opacity: 1    },
    neighbor: { lineWidth: 3,   stroke: 'rgba(139,92,246,0.65)',  shadowColor: 'rgba(0,0,0,0.10)',       shadowBlur: 14, opacity: 1    },
    dimmed:   { lineWidth: 1,   stroke: 'rgba(255,255,255,0.9)',  shadowColor: 'rgba(0,0,0,0.10)',       shadowBlur: 14, opacity: 0.35 },
  }[role];

  const style = { ...base, fill };
  if (hover && role !== 'selected') {
    style.lineWidth = 3;
    style.stroke = 'rgba(255,255,255,0.95)';
    style.shadowColor = 'rgba(15,23,42,0.18)';
    style.shadowBlur = 18;
    if (role === 'dimmed') style.opacity = 0.7;
  }
  return { style };
}

const searchQuery = ref('');
const showSearchResults = ref(false);

const statsText = computed(() => `${graphNodes.value.length} 人物 · ${graphEdges.value.length} 关系`);

const idToCharacter = computed(() => {
  const map = new Map<string, CharacterInfo>();
  for (const c of characters.value) map.set(c.id, c);
  return map;
});

function normalizeKey(v: string): string {
  return (v || '')
    .trim()
    .toLowerCase()
    .replace(/[\s\u3000]+/g, '')
    .replace(/[·•・．\.\u3001,，:：;；!！?？()（）\[\]{}《》<>"'“”‘’]/g, '');
}

function roleColor(role: string): string {
  switch (role) {
    case 'protagonist':
      return '#f59e0b'; // amber-500
    case 'antagonist':
      return '#ef4444'; // red-500
    case 'major':
      return '#8b5cf6'; // purple-500
    case 'supporting':
      return '#06b6d4'; // cyan-500
    case 'minor':
    default:
      return '#94a3b8'; // gray-400
  }
}

function displayLabel(name: string): string {
  const max = 6;
  if (!name) return '';
  return name.length > max ? `${name.slice(0, max)}...` : name;
}

function escapeHtml(s: string) {
  return (s || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

type ParsedRelation = { targetRaw: string; label: string };

function parseRelationships(value: unknown): ParsedRelation[] {
  if (!value) return [];

  if (typeof value === 'string') {
    const trimmed = value.trim();
    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
      try {
        return parseRelationships(JSON.parse(trimmed) as unknown);
      } catch {
        return [];
      }
    }
    return [];
  }

  if (typeof value !== 'object') return [];

  // Array form: [{ target_id, relation_type, description }, ...]
  if (Array.isArray(value)) {
    const out: ParsedRelation[] = [];
    for (const item of value) {
      if (!item || typeof item !== 'object') continue;
      const obj = item as Record<string, unknown>;
      const targetRaw =
        (typeof obj.target_id === 'string' && obj.target_id) ||
        (typeof obj.targetId === 'string' && obj.targetId) ||
        (typeof obj.target === 'string' && obj.target) ||
        (typeof obj.name === 'string' && obj.name) ||
        (typeof obj.target_name === 'string' && obj.target_name) ||
        '';
      if (!targetRaw) continue;

      const label =
        (typeof obj.relation_type === 'string' && obj.relation_type) ||
        (typeof obj.relationType === 'string' && obj.relationType) ||
        (typeof obj.type === 'string' && obj.type) ||
        (typeof obj.relation === 'string' && obj.relation) ||
        (typeof obj.description === 'string' && obj.description) ||
        '关系';
      out.push({ targetRaw, label });
    }
    return out;
  }

  const obj = value as Record<string, unknown>;

  // Single relationship object: { target_id, relation_type, ... }
  if (
    typeof obj.target_id === 'string' ||
    typeof obj.target === 'string' ||
    typeof obj.targetId === 'string'
  ) {
    const targetRaw =
      (typeof obj.target_id === 'string' && obj.target_id) ||
      (typeof obj.targetId === 'string' && obj.targetId) ||
      (typeof obj.target === 'string' && obj.target) ||
      '';
    const label =
      (typeof obj.relation_type === 'string' && obj.relation_type) ||
      (typeof obj.relationType === 'string' && obj.relationType) ||
      (typeof obj.type === 'string' && obj.type) ||
      (typeof obj.relation === 'string' && obj.relation) ||
      (typeof obj.description === 'string' && obj.description) ||
      '关系';
    return targetRaw ? [{ targetRaw, label }] : [];
  }

  // Common map form: { "Bob": "朋友", "Charlie": { type: "父亲" } }
  const out: ParsedRelation[] = [];
  for (const [k, v] of Object.entries(obj)) {
    if (!k) continue;
    if (typeof v === 'string') {
      out.push({ targetRaw: k, label: v.trim() || '关系' });
      continue;
    }
    if (Array.isArray(v)) {
      const parts = v.map(x => (typeof x === 'string' ? x.trim() : '')).filter(Boolean);
      out.push({ targetRaw: k, label: parts.join(' / ') || '关系' });
      continue;
    }
    if (v && typeof v === 'object') {
      const vObj = v as Record<string, unknown>;
      const label =
        (typeof vObj.relation_type === 'string' && vObj.relation_type) ||
        (typeof vObj.relationType === 'string' && vObj.relationType) ||
        (typeof vObj.type === 'string' && vObj.type) ||
        (typeof vObj.relation === 'string' && vObj.relation) ||
        (typeof vObj.description === 'string' && vObj.description) ||
        (typeof vObj.label === 'string' && vObj.label) ||
        '关系';
      out.push({ targetRaw: k, label });
      continue;
    }
    out.push({ targetRaw: k, label: '关系' });
  }
  return out;
}

function buildGraphModel() {
  const chars = characters.value;
  const idToChar = new Map<string, CharacterInfo>();
  const normToIds = new Map<string, string[]>();

  for (const c of chars) {
    idToChar.set(c.id, c);
    const keys = [c.name, ...(c.aliases || [])].filter(Boolean);
    for (const k of keys) {
      const norm = normalizeKey(k);
      if (!norm) continue;
      const arr = normToIds.get(norm) || [];
      if (!arr.includes(c.id)) arr.push(c.id);
      normToIds.set(norm, arr);
    }
  }

  function resolveTargetId(targetRaw: string): { id?: string; reason?: UnresolvedRelation['reason']; candidates?: string[] } {
    const raw = (targetRaw || '').trim();
    if (!raw) return { reason: 'not_found' };

    const candidateStrings = (() => {
      const list: string[] = [raw];

      // Strip bracketed notes: "张三(父亲)" / "张三（父亲）" / "张三【父亲】"
      const stripped = raw
        .replace(/\(.*?\)/g, '')
        .replace(/（.*?）/g, '')
        .replace(/【.*?】/g, '')
        .replace(/\[.*?\]/g, '')
        .replace(/《.*?》/g, '')
        .trim();
      if (stripped && stripped !== raw) list.push(stripped);

      // Split on common separators: "张三 - 李四", "张三：李四"
      const split = stripped.split(/[-—–:：,，\/、·•]/).map(s => s.trim()).filter(Boolean);
      if (split.length > 1) {
        if (split[0] && !list.includes(split[0])) list.push(split[0]);
      }

      return list;
    })();

    // Prefer the first unique match across candidates to avoid wrong links.
    for (const cand of candidateStrings) {
      if (idToChar.has(cand)) return { id: cand };
      const norm = normalizeKey(cand);
      if (!norm) continue;
      const ids = normToIds.get(norm);
      if (ids && ids.length === 1) return { id: ids[0] };
    }

    // If any candidate is ambiguous, surface it; otherwise not found.
    for (const cand of candidateStrings) {
      const norm = normalizeKey(cand);
      if (!norm) continue;
      const ids = normToIds.get(norm);
      if (ids && ids.length > 1) {
        const cands = ids
          .map(id => idToChar.get(id)?.name)
          .filter((n): n is string => Boolean(n));
        return { reason: 'ambiguous', candidates: cands };
      }
    }

    return { reason: 'not_found' };
  }

  type PairInternal = {
    aId: string;
    bId: string;
    aToB: Set<string>;
    bToA: Set<string>;
  };

  const pairMap = new Map<string, PairInternal>();
  const unresolvedList: UnresolvedRelation[] = [];

  for (const source of chars) {
    const rels = parseRelationships(source.relationships);
    for (const rel of rels) {
      const label = (rel.label || '').trim() || '关系';
      const resolved = resolveTargetId(rel.targetRaw);

      if (!resolved.id) {
        unresolvedList.push({
          sourceId: source.id,
          sourceName: source.name,
          targetRaw: rel.targetRaw,
          relation: label,
          reason: resolved.reason || 'not_found',
          candidates: resolved.candidates,
        });
        continue;
      }

      const targetId = resolved.id;
      if (targetId === source.id) continue;

      const aId = source.id < targetId ? source.id : targetId;
      const bId = source.id < targetId ? targetId : source.id;
      const key = `${aId}--${bId}`;

      let pair = pairMap.get(key);
      if (!pair) {
        pair = { aId, bId, aToB: new Set<string>(), bToA: new Set<string>() };
        pairMap.set(key, pair);
      }

      if (source.id === aId) {
        pair.aToB.add(label);
      } else {
        pair.bToA.add(label);
      }
    }
  }

  // Populate standalone fill color map (G6-proof source of truth).
  nodeFillColors.clear();
  const nodeList = chars.map(c => {
    const fill = roleColor(c.role);
    nodeFillColors.set(c.id, fill);
    return {
      id: c.id,
      label: displayLabel(c.name),
      fullLabel: c.name,
      role: c.role,
      size: BASE_NODE_SIZE,
      style: { fill },
    };
  });

  const pairs: PairRelation[] = Array.from(pairMap.values()).map(p => ({
    aId: p.aId,
    bId: p.bId,
    aToB: Array.from(p.aToB.values()).sort(),
    bToA: Array.from(p.bToA.values()).sort(),
  }));

  const edges = pairs.map(p => ({
    id: `${p.aId}--${p.bId}`,
    source: p.aId,
    target: p.bId,
    relations: p,
    style: {
      stroke: '#94a3b8',
      lineWidth: Math.min(1 + (p.aToB.length + p.bToA.length) * 0.35, 3),
      opacity: 0.7,
    },
  }));

  graphNodes.value = nodeList;
  graphEdges.value = edges;
  pairRelations.value = pairs;
  unresolved.value = unresolvedList;
}

function ensureGraph() {
  if (!graphContainerRef.value || graph) return;

  const container = graphContainerRef.value;
  const rect = container.getBoundingClientRect();
  const width = Math.max(320, Math.floor(rect.width));
  const height = Math.max(320, Math.floor(rect.height));

  const tooltip = new G6Any.Tooltip({
    offsetX: 12,
    offsetY: 12,
    itemTypes: ['node', 'edge'],
    getContent: (e: any) => {
      const div = document.createElement('div');
      div.style.padding = '8px 10px';
      div.style.maxWidth = '280px';
      div.style.fontSize = '12px';
      div.style.lineHeight = '1.4';
      div.style.color = '#0f172a';

      const item = e?.item;
      if (!item) return div;

      const model = item.getModel?.() || {};
      const type = item.getType?.();

      if (type === 'node') {
        const id = model.id as string;
        const char = idToCharacter.value.get(id);
        const title = char?.name || (model.fullLabel as string) || (model.label as string) || id;
        const role = char?.role || model.role || '';
        div.innerHTML = `<div style="font-weight:600; margin-bottom:4px;">${escapeHtml(title)}</div>
          <div style="opacity:0.7;">${escapeHtml(String(role))}</div>`;
        return div;
      }

      if (type === 'edge') {
        const rel = (model.relations || {}) as PairRelation;
        const a = idToCharacter.value.get(rel.aId)?.name || rel.aId;
        const b = idToCharacter.value.get(rel.bId)?.name || rel.bId;

        const aToB = (rel.aToB || []).map(escapeHtml).join(' / ');
        const bToA = (rel.bToA || []).map(escapeHtml).join(' / ');

        const lines: string[] = [];
        lines.push(
          `<div style="font-weight:600; margin-bottom:4px;">${escapeHtml(a)} <span style="opacity:0.6;">&</span> ${escapeHtml(b)}</div>`
        );
        if (aToB) lines.push(`<div><span style="opacity:0.7;">${escapeHtml(a)} -> ${escapeHtml(b)}:</span> ${aToB}</div>`);
        if (bToA) lines.push(`<div><span style="opacity:0.7;">${escapeHtml(b)} -> ${escapeHtml(a)}:</span> ${bToA}</div>`);
        if (!aToB && !bToA) lines.push(`<div style="opacity:0.7;">关系</div>`);
        div.innerHTML = lines.join('');
        return div;
      }

      return div;
    },
  });

  const minimap = new G6Any.Minimap({
    size: [180, 120],
    className: 'g6-minimap',
    type: 'delegate',
  });

  graph = new G6Any.Graph({
    container,
    width,
    height,
    renderer: 'canvas',
    animate: true,
    fitView: true,
    fitViewPadding: 32,
    layout: {
      type: 'force',
      preventOverlap: true,
      nodeSize: BASE_NODE_SIZE,
      nodeSpacing: 12,
      linkDistance: 140,
      edgeStrength: 0.2,
      nodeStrength: -260,
      damping: 0.9,
    },
    defaultNode: {
      type: 'circle',
      size: BASE_NODE_SIZE,
      style: {
        lineWidth: 1,
        stroke: 'rgba(255,255,255,0.9)',
        shadowColor: 'rgba(0,0,0,0.10)',
        shadowBlur: 14,
        cursor: 'pointer',
      },
      labelCfg: {
        position: 'center',
        style: {
          fill: '#ffffff',
          fontSize: 12,
          fontWeight: 600,
        },
      },
    },
    defaultEdge: {
      type: 'line',
      style: {
        stroke: '#94a3b8',
        lineWidth: 1.4,
        opacity: 0.7,
      },
    },
    edgeStateStyles: {
      hover: { opacity: 1, lineWidth: 2 },
      active: {
        stroke: '#8b5cf6',
        opacity: 1,
        lineWidth: 2.4,
      },
      inactive: { opacity: 0.22 },
    },
    modes: {
      default: [
        'drag-canvas',
        'zoom-canvas',
        {
          type: 'drag-node',
          enableDelegate: false,
        },
      ],
    },
    plugins: [tooltip, minimap],
  });

  graph.on('node:mouseenter', (evt: any) => {
    if (!graph || !evt?.item) return;
    const id = evt.item.getID() as string;
    const role = nodeVisualRoles.get(id) ?? 'default';
    graph.updateItem(evt.item, getNodeStyle(id, role, true));
  });
  graph.on('node:mouseleave', (evt: any) => {
    if (!graph || !evt?.item) return;
    const id = evt.item.getID() as string;
    const role = nodeVisualRoles.get(id) ?? 'default';
    graph.updateItem(evt.item, getNodeStyle(id, role, false));
  });
  graph.on('edge:mouseenter', (evt: any) => {
    if (!graph) return;
    if (evt?.item) graph.setItemState(evt.item, 'hover', true);
  });
  graph.on('edge:mouseleave', (evt: any) => {
    if (!graph) return;
    if (evt?.item) graph.setItemState(evt.item, 'hover', false);
  });

  graph.on('node:click', (evt: any) => {
    const id = evt?.item?.getID?.() as string | undefined;
    if (!id) return;
    selectedId.value = id;
    applySelectionStates();
  });

  graph.on('canvas:click', () => {
    selectedId.value = null;
    applySelectionStates();
  });

  resizeObserver = new ResizeObserver(() => {
    if (!graph || !graphContainerRef.value) return;
    const r = graphContainerRef.value.getBoundingClientRect();
    const w = Math.max(320, Math.floor(r.width));
    const h = Math.max(320, Math.floor(r.height));
    graph.changeSize(w, h);
  });
  resizeObserver.observe(container);
}

function renderGraph() {
  if (!graph) return;

  if (selectedId.value && !graphNodes.value.some(n => n.id === selectedId.value)) {
    selectedId.value = null;
  }

  const data = { nodes: graphNodes.value, edges: graphEdges.value };

  if (graph.getNodes().length === 0 && graph.getEdges().length === 0) {
    graph.data(data);
    graph.render();
  } else {
    graph.changeData(data);
  }

  graph.layout();
  applySelectionStates();
}

function applySelectionStates() {
  if (!graph) return;

  const current = selectedId.value;
  const nodes = graph.getNodes();
  const edges = graph.getEdges();

  // Reset edge G6 states (edges are safe to use setItemState).
  const edgeStates = ['hover', 'active', 'inactive'] as const;
  for (const e of edges) {
    for (const s of edgeStates) graph.setItemState(e, s, false);
  }

  // No selection: restore all nodes to default.
  if (!current) {
    if (lastSelectedSizeId) {
      const prev = graph.findById(lastSelectedSizeId);
      if (prev) graph.updateItem(prev, { size: BASE_NODE_SIZE, ...getNodeStyle(lastSelectedSizeId, 'default') });
      lastSelectedSizeId = null;
    }
    nodeVisualRoles.clear();
    for (const n of nodes) {
      const id = n.getID();
      nodeVisualRoles.set(id, 'default');
      graph.updateItem(n, getNodeStyle(id, 'default'));
    }
    return;
  }

  // Shrink previously enlarged node.
  if (lastSelectedSizeId && lastSelectedSizeId !== current) {
    const prev = graph.findById(lastSelectedSizeId);
    if (prev) graph.updateItem(prev, { size: BASE_NODE_SIZE, ...getNodeStyle(lastSelectedSizeId, 'default') });
  }

  // Enlarge + style selected node in one call.
  const selectedItem = graph.findById(current);
  if (selectedItem) {
    graph.updateItem(selectedItem, { size: SELECTED_NODE_SIZE, ...getNodeStyle(current, 'selected') });
    (selectedItem as any).toFront?.();
    graph.toFront?.(selectedItem);
  }
  lastSelectedSizeId = current;

  // Determine neighbors via edges. If the selected node has no edges, do not dim the rest.
  const neighbors = new Set<string>();
  const connectedEdges: any[] = [];
  const otherEdges: any[] = [];

  for (const e of edges) {
    const m = e.getModel() as any;
    const s = m.source as string;
    const t = m.target as string;
    if (s === current || t === current) {
      connectedEdges.push(e);
      neighbors.add(s === current ? t : s);
    } else {
      otherEdges.push(e);
    }
  }

  // Update nodeVisualRoles and apply styles for all nodes.
  nodeVisualRoles.clear();
  if (connectedEdges.length === 0) {
    // No connected edges: keep all visible.
    for (const n of nodes) {
      const id = n.getID();
      const role: NodeRole = id === current ? 'selected' : 'default';
      nodeVisualRoles.set(id, role);
      if (id !== current) graph.updateItem(n, getNodeStyle(id, 'default'));
    }
    return;
  }

  // Edge states.
  for (const e of connectedEdges) graph.setItemState(e, 'active', true);
  for (const e of otherEdges) graph.setItemState(e, 'inactive', true);

  // Node styles via updateItem only — no setItemState for nodes.
  for (const n of nodes) {
    const id = n.getID();
    if (id === current) {
      nodeVisualRoles.set(id, 'selected');
      // Already styled above with size change.
    } else if (neighbors.has(id)) {
      nodeVisualRoles.set(id, 'neighbor');
      graph.updateItem(n, getNodeStyle(id, 'neighbor'));
    } else {
      nodeVisualRoles.set(id, 'dimmed');
      graph.updateItem(n, getNodeStyle(id, 'dimmed'));
    }
  }
}

function focus(characterId: string) {
  if (!characterId) return;
  selectedId.value = characterId;

  if (!graph) return;

  const item = graph.findById(characterId);
  if (!item) return;

  graph.focusItem(item, true, {
    easing: 'easeCubic',
    duration: 450,
  });
  applySelectionStates();
}

function fitView() {
  if (!graph) return;
  graph.fitView(32);
}

function relayout() {
  if (!graph) return;
  graph.layout();
}

const selectedCharacter = computed(() => {
  if (!selectedId.value) return null;
  return characters.value.find(c => c.id === selectedId.value) || null;
});

const selectedConnections = computed(() => {
  if (!selectedId.value) return [];

  const id = selectedId.value;
  const results: Array<{
    otherId: string;
    otherName: string;
    outgoing: string[];
    incoming: string[];
  }> = [];

  for (const p of pairRelations.value) {
    if (p.aId !== id && p.bId !== id) continue;

    const otherId = p.aId === id ? p.bId : p.aId;
    const otherName = idToCharacter.value.get(otherId)?.name || otherId;

    const outgoing = p.aId === id ? p.aToB : p.bToA;
    const incoming = p.aId === id ? p.bToA : p.aToB;

    results.push({ otherId, otherName, outgoing, incoming });
  }

  results.sort((a, b) => {
    const aw = a.outgoing.length + a.incoming.length;
    const bw = b.outgoing.length + b.incoming.length;
    if (bw !== aw) return bw - aw;
    return a.otherName.localeCompare(b.otherName);
  });

  return results;
});

const searchResults = computed(() => {
  const q = searchQuery.value.trim();
  if (!q) return [];

  const normQ = normalizeKey(q);
  if (!normQ) return [];

  return characters.value
    .filter(c => {
      const keys = [c.name, ...(c.aliases || [])].filter(Boolean);
      return keys.some(k => normalizeKey(k).includes(normQ));
    })
    .slice(0, 8)
    .map(c => ({ id: c.id, name: c.name, role: c.role }));
});

function handleSearchBlur() {
  window.setTimeout(() => {
    showSearchResults.value = false;
  }, 120);
}

function handleSearchSelect(id: string) {
  searchQuery.value = '';
  showSearchResults.value = false;
  focus(id);
}

async function loadData() {
  if (!props.bookId) return;

  loading.value = true;
  error.value = null;

  try {
    const data = await invoke<CharacterInfo[]>('get_characters', { bookId: props.bookId });
    characters.value = data || [];
    buildGraphModel();
    await nextTick();
    ensureGraph();
    renderGraph();
  } catch (e) {
    console.error('Failed to load character relationship data:', e);
    error.value = typeof e === 'string' ? e : '加载人物关系失败';
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  await nextTick();
  ensureGraph();
  await loadData();
});

watch(() => props.bookId, loadData);
watch(selectedId, applySelectionStates);

onBeforeUnmount(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
  if (graph) {
    graph.destroy();
    graph = null;
  }
});

defineExpose({
  refresh: loadData,
  focus,
});
</script>

<template>
  <div class="h-full flex flex-col bg-white dark:bg-gray-900">
    <!-- Header -->
    <div class="flex items-center justify-between gap-4 px-6 py-4 border-b border-gray-200 dark:border-gray-700">
      <!-- View Mode Switcher + Stats -->
      <div class="flex items-center gap-3">
        <div class="inline-flex items-center gap-1 p-1 bg-fabric-sand/20 rounded-full">
          <button
            @click="emit('update:viewMode', 'timeline')"
            class="flex items-center gap-2 px-3.5 py-1.5 text-sm font-medium rounded-full transition-all duration-220 text-fabric-thread hover:text-fabric-sepia"
          >
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 17h8m0 0V9m0 8l-8-8-4 4-6-6" />
            </svg>
            时间线
          </button>
          <button
            :class="[
              'flex items-center gap-2 px-3.5 py-1.5 text-sm font-medium rounded-full transition-all duration-220',
              'bg-accent-character/15 text-accent-character shadow-sm'
            ]"
          >
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
            人物关系
          </button>
        </div>
        <span class="text-xs text-gray-500 dark:text-gray-400">
          {{ statsText }}
          <span v-if="unresolved.length" class="ml-1 text-amber-600 dark:text-amber-400">
            · 未解析 {{ unresolved.length }} 条
          </span>
        </span>
      </div>

      <div class="flex items-center gap-2 flex-shrink-0">
        <!-- Search -->
        <div class="relative">
          <input
            v-model="searchQuery"
            @focus="showSearchResults = true"
            @blur="handleSearchBlur"
            @keydown.escape="showSearchResults = false"
            class="w-56 px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-400 focus:outline-none focus:ring-2 focus:ring-purple-500/40"
            placeholder="搜索人物 (支持别名)"
          />
          <div
            v-if="showSearchResults && searchResults.length"
            class="absolute right-0 mt-1 w-72 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl shadow-lg overflow-hidden z-10"
          >
            <button
              v-for="r in searchResults"
              :key="r.id"
              @click="handleSearchSelect(r.id)"
              class="w-full text-left px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700/60 transition-colors"
            >
              <div class="flex items-center justify-between gap-2">
                <span class="text-sm text-gray-900 dark:text-gray-100">{{ r.name }}</span>
                <span class="text-[11px] text-gray-500 dark:text-gray-400">{{ r.role }}</span>
              </div>
            </button>
          </div>
        </div>

        <!-- Actions -->
        <button
          @click="fitView"
          class="px-3 py-2 text-xs font-medium rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          title="适应屏幕"
        >
          适应
        </button>
        <button
          @click="relayout"
          class="px-3 py-2 text-xs font-medium rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          title="重新布局"
        >
          重排
        </button>
      </div>
    </div>

    <!-- Body -->
    <div class="flex-1 min-h-0 grid grid-cols-1 lg:grid-cols-[1fr_320px]">
      <!-- Graph -->
      <div class="relative overflow-hidden">
        <div ref="graphContainerRef" class="absolute inset-0 graph-surface"></div>

        <!-- Loading -->
        <div v-if="loading" class="absolute inset-0 flex items-center justify-center bg-white/70 dark:bg-gray-900/70">
          <div class="flex flex-col items-center gap-3">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-500"></div>
            <span class="text-sm text-gray-500">加载人物关系...</span>
          </div>
        </div>

        <!-- Error -->
        <div v-else-if="error" class="absolute inset-0 flex items-center justify-center">
          <div class="text-center">
            <p class="text-red-500 mb-2">{{ error }}</p>
            <button @click="loadData" class="text-sm text-purple-500 hover:text-purple-600">重试</button>
          </div>
        </div>

        <!-- Empty -->
        <div v-else-if="graphNodes.length === 0" class="absolute inset-0 flex items-center justify-center">
          <div class="text-center text-gray-400">
            <svg class="w-16 h-16 mx-auto mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
            <p class="text-lg font-medium">暂无人物关系</p>
            <p class="text-sm mt-1">分析章节或在人物资料中补充关系后，将在这里展示</p>
          </div>
        </div>

        <!-- Mobile Selected Panel -->
        <div
          v-if="selectedCharacter && !loading && !error && graphNodes.length"
          class="lg:hidden absolute bottom-4 left-4 right-4 bg-white/95 dark:bg-gray-900/85 backdrop-blur rounded-2xl shadow-lg border border-gray-200 dark:border-gray-800 p-4"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="flex items-center gap-3 min-w-0">
              <div
                class="w-10 h-10 rounded-full flex items-center justify-center text-white font-bold flex-shrink-0"
                :style="{ backgroundColor: roleColor(selectedCharacter.role) }"
              >
                {{ selectedCharacter.name.charAt(0) }}
              </div>
              <div class="min-w-0">
                <div class="text-sm font-bold text-gray-900 dark:text-gray-100 truncate">
                  {{ selectedCharacter.name }}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400">{{ selectedCharacter.role }}</div>
              </div>
            </div>
            <button
              class="px-3 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-gray-800 text-gray-600 dark:text-gray-300 hover:bg-white dark:hover:bg-gray-900 transition-colors flex-shrink-0"
              @click="selectedId = null"
            >
              关闭
            </button>
          </div>

          <div class="mt-3">
            <div v-if="selectedConnections.length" class="flex flex-wrap gap-1.5">
              <button
                v-for="c in selectedConnections.slice(0, 8)"
                :key="c.otherId"
                class="px-2.5 py-1 text-xs rounded-full border border-gray-200 dark:border-gray-800 bg-white/70 dark:bg-gray-900/40 text-gray-700 dark:text-gray-200 hover:bg-white dark:hover:bg-gray-900 transition-colors"
                @click="focus(c.otherId)"
              >
                {{ c.otherName }}
              </button>
            </div>
            <div v-else class="text-xs text-gray-500 dark:text-gray-400">暂无已记录的关系</div>
          </div>
        </div>
      </div>

      <!-- Side Panel -->
      <div class="hidden lg:flex flex-col border-l border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900">
        <div class="p-4 border-b border-gray-200 dark:border-gray-800">
          <h4 class="text-sm font-semibold text-gray-900 dark:text-gray-100">详情</h4>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">点击人物节点查看其关联关系</p>
        </div>

        <div class="flex-1 overflow-y-auto p-4 space-y-4">
          <!-- Selected -->
          <div
            v-if="selectedCharacter"
            class="rounded-xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-800/40 p-4"
          >
            <div class="flex items-center gap-3">
              <div
                class="w-10 h-10 rounded-full flex items-center justify-center text-white font-bold"
                :style="{ backgroundColor: roleColor(selectedCharacter.role) }"
              >
                {{ selectedCharacter.name.charAt(0) }}
              </div>
              <div class="min-w-0">
                <div class="text-sm font-bold text-gray-900 dark:text-gray-100 truncate">
                  {{ selectedCharacter.name }}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400">{{ selectedCharacter.role }}</div>
              </div>
            </div>

            <div v-if="selectedConnections.length" class="mt-4">
              <div class="text-xs font-medium text-gray-600 dark:text-gray-300 mb-2">关联人物</div>
              <div class="space-y-2">
                <button
                  v-for="c in selectedConnections"
                  :key="c.otherId"
                  class="w-full text-left px-3 py-2 rounded-lg border border-gray-200 dark:border-gray-800 bg-white/80 dark:bg-gray-900/40 hover:bg-white dark:hover:bg-gray-900 transition-colors"
                  @click="focus(c.otherId)"
                >
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-sm text-gray-900 dark:text-gray-100 truncate">{{ c.otherName }}</span>
                    <span class="text-[11px] text-gray-400">{{ c.outgoing.length + c.incoming.length }} 条</span>
                  </div>
                  <div class="mt-1 flex flex-wrap gap-1">
                    <span
                      v-for="(t, idx) in c.outgoing"
                      :key="'out-' + idx"
                      class="px-2 py-0.5 text-[11px] rounded-full bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300"
                      title="我 -> 对方"
                    >
                      {{ t }}
                    </span>
                    <span
                      v-for="(t, idx) in c.incoming"
                      :key="'in-' + idx"
                      class="px-2 py-0.5 text-[11px] rounded-full bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300"
                      title="对方 -> 我"
                    >
                      {{ t }}
                    </span>
                  </div>
                </button>
              </div>
            </div>
            <div v-else class="mt-4 text-xs text-gray-500 dark:text-gray-400">
              暂无已记录的关系（可通过 AI 分析或后续关系编辑器补充）
            </div>

            <button
              class="mt-4 w-full px-3 py-2 text-xs rounded-lg border border-gray-200 dark:border-gray-800 text-gray-600 dark:text-gray-300 hover:bg-white dark:hover:bg-gray-900 transition-colors"
              @click="selectedId = null"
            >
              取消选择
            </button>
          </div>

          <!-- Unresolved -->
          <div
            v-if="unresolved.length"
            class="rounded-xl border border-amber-200 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-900/10 p-4"
          >
            <div class="text-sm font-semibold text-amber-800 dark:text-amber-200">未解析关系 ({{ unresolved.length }})</div>
            <div class="mt-3 space-y-2 max-h-64 overflow-y-auto pr-1">
              <div
                v-for="(u, idx) in unresolved.slice(0, 30)"
                :key="idx"
                class="text-xs text-amber-900/80 dark:text-amber-200/80"
              >
                <span class="font-medium">{{ u.sourceName }}</span>
                <span class="opacity-70"> -> </span>
                <span class="font-medium">{{ u.targetRaw }}</span>
                <span class="opacity-70">:</span>
                <span class="ml-1">{{ u.relation }}</span>
                <span v-if="u.reason === 'ambiguous'" class="ml-1 opacity-70">(重名/别名冲突)</span>
              </div>
              <div v-if="unresolved.length > 30" class="text-[11px] text-amber-900/60 dark:text-amber-200/60">
                仅显示前 30 条...
              </div>
            </div>
            <div class="mt-3 text-[11px] text-amber-900/70 dark:text-amber-200/70">
              提示：如果关系里使用了别名或有重名人物，建议统一命名或在人物别名中补齐。
            </div>
          </div>

          <!-- Legend -->
          <div class="rounded-xl border border-gray-200 dark:border-gray-800 p-4">
            <div class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">图例</div>
            <div class="space-y-2 text-xs">
              <div class="flex items-center gap-2">
                <div class="w-3 h-3 rounded-full" :style="{ backgroundColor: roleColor('protagonist') }"></div>
                <span class="text-gray-600 dark:text-gray-400">主角</span>
              </div>
              <div class="flex items-center gap-2">
                <div class="w-3 h-3 rounded-full" :style="{ backgroundColor: roleColor('antagonist') }"></div>
                <span class="text-gray-600 dark:text-gray-400">反派</span>
              </div>
              <div class="flex items-center gap-2">
                <div class="w-3 h-3 rounded-full" :style="{ backgroundColor: roleColor('major') }"></div>
                <span class="text-gray-600 dark:text-gray-400">主要角色</span>
              </div>
              <div class="flex items-center gap-2">
                <div class="w-3 h-3 rounded-full" :style="{ backgroundColor: roleColor('supporting') }"></div>
                <span class="text-gray-600 dark:text-gray-400">配角</span>
              </div>
              <div class="flex items-center gap-2">
                <div class="w-3 h-3 rounded-full" :style="{ backgroundColor: roleColor('minor') }"></div>
                <span class="text-gray-600 dark:text-gray-400">龙套</span>
              </div>
              <div class="mt-3 text-[11px] text-gray-500 dark:text-gray-400">
                支持拖拽节点 / 滚轮缩放 / 拖动画布
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.graph-surface {
  background:
    radial-gradient(circle at 1px 1px, rgba(148, 163, 184, 0.22) 1px, transparent 0) 0 0 / 18px 18px,
    linear-gradient(180deg, rgba(248, 250, 252, 0.92), rgba(255, 255, 255, 1));
}

.dark .graph-surface {
  background:
    radial-gradient(circle at 1px 1px, rgba(148, 163, 184, 0.14) 1px, transparent 0) 0 0 / 18px 18px,
    linear-gradient(180deg, rgba(17, 24, 39, 0.95), rgba(3, 7, 18, 1));
}

:deep(.g6-minimap) {
  box-shadow: 0 8px 24px rgba(15, 23, 42, 0.12);
  border: 1px solid rgba(226, 232, 240, 0.9);
  border-radius: 10px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.85);
}

.dark :deep(.g6-minimap) {
  border: 1px solid rgba(148, 163, 184, 0.25);
  background: rgba(17, 24, 39, 0.75);
}
</style>
