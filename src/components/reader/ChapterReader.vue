<script setup lang="ts">
import { computed, watch, ref, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useBookStore } from '@/stores/book';
import { useAnalysisStore } from '@/stores/analysis';
import TextHighlight, { type HighlightRange } from './TextHighlight.vue';
import EvidenceModal from './EvidenceModal.vue';

// Entity types for highlighting
interface EntityBase {
  id: string;
  name: string;
  aliases: string[];
  description?: string;
}

interface CharacterEntity extends EntityBase {
  type: 'character';
  role: string;
  traits: string[];
}

interface SettingEntity extends EntityBase {
  type: 'setting';
  setting_type: string;
}

interface EventEntity extends EntityBase {
  type: 'event';
  event_type?: string;
}

type Entity = CharacterEntity | SettingEntity | EventEntity;

// Backend data shapes (from src-tauri/src/commands/bible.rs)
interface BackendCharacterInfo {
  id: string;
  name: string;
  aliases: string[];
  description?: string | null;
  traits: string[];
  role: string;
}

interface BackendSettingInfo {
  id: string;
  setting_type: string;
  name: string;
  description?: string | null;
}

interface BackendEventInfo {
  id: string;
  title: string;
  description?: string | null;
}

interface EntityHighlight extends HighlightRange {
  entityId: string;
  entityType: 'character' | 'setting' | 'event';
  entityName: string;
}

const bookStore = useBookStore();
const analysisStore = useAnalysisStore();

const readerContent = ref<HTMLElement | null>(null);

// Entity highlighting state
const showEntityHighlights = ref(false);
const entities = ref<Entity[]>([]);
const loadingEntities = ref(false);
const selectedEntity = ref<Entity | null>(null);

// Entity colors
const entityColors = {
  character: 'rgba(139, 92, 246, 0.25)', // purple
  setting: 'rgba(59, 130, 246, 0.25)',   // blue
  event: 'rgba(249, 115, 22, 0.25)',     // orange
};

const entityBorderColors = {
  character: '#8b5cf6',
  setting: '#3b82f6',
  event: '#f97316',
};

// Load entities for the current book
const loadEntities = async () => {
  if (!bookStore.currentBook?.id) return;

  loadingEntities.value = true;
  try {
    const bookId = String(bookStore.currentBook.id);

    // Load all entity types in parallel
    const [characters, settings, events] = await Promise.all([
      invoke<BackendCharacterInfo[]>('get_characters', { bookId }),
      invoke<BackendSettingInfo[]>('get_settings', { bookId }),
      invoke<BackendEventInfo[]>('get_events', { bookId }),
    ]);

    entities.value = [
      ...characters.map((c): CharacterEntity => ({
        id: c.id,
        name: c.name,
        aliases: Array.isArray(c.aliases) ? c.aliases : [],
        description: c.description ?? undefined,
        type: 'character',
        role: c.role,
        traits: Array.isArray(c.traits) ? c.traits : [],
      })),
      ...settings.map((s): SettingEntity => ({
        id: s.id,
        name: s.name,
        aliases: [],
        description: s.description ?? undefined,
        type: 'setting',
        setting_type: s.setting_type,
      })),
      ...events.map((e): EventEntity => ({
        id: e.id,
        // EventInfo uses `title`, but our highlighter expects `name`
        name: e.title,
        aliases: [],
        description: e.description ?? undefined,
        type: 'event',
      })),
    ];
  } catch (e) {
    console.error('Failed to load entities:', e);
  } finally {
    loadingEntities.value = false;
  }
};

// Find entity mentions in text
const findEntityMentions = (text: string): EntityHighlight[] => {
  if (!showEntityHighlights.value || entities.value.length === 0) {
    return [];
  }

  const mentions: EntityHighlight[] = [];

  // Sort entities by name length (longer first) to avoid partial matches
  const sortedEntities = [...entities.value].sort((a, b) => {
    const aAliasLens = Array.isArray(a.aliases) ? a.aliases.map(alias => alias.length) : [];
    const bAliasLens = Array.isArray(b.aliases) ? b.aliases.map(alias => alias.length) : [];
    const aMaxLen = Math.max(0, (a as { name?: string }).name?.length ?? 0, ...aAliasLens);
    const bMaxLen = Math.max(0, (b as { name?: string }).name?.length ?? 0, ...bAliasLens);
    return bMaxLen - aMaxLen;
  });

  // Track used ranges to avoid overlapping highlights
  const usedRanges: Array<{ start: number; end: number }> = [];

  const isOverlapping = (start: number, end: number): boolean => {
    return usedRanges.some(range =>
      (start >= range.start && start < range.end) ||
      (end > range.start && end <= range.end) ||
      (start <= range.start && end >= range.end)
    );
  };

  for (const entity of sortedEntities) {
    const namesToSearch = [entity.name, ...(entity.aliases || [])].filter(n => n && n.length >= 2);

    for (const name of namesToSearch) {
      let searchPos = 0;
      while (true) {
        const index = text.indexOf(name, searchPos);
        if (index === -1) break;

        const end = index + name.length;

        // Check if this is a word boundary (avoid matching partial words)
        const prevChar = index > 0 ? text[index - 1] : ' ';
        const nextChar = end < text.length ? text[end] : ' ';
        const isWordBoundary = /[\s，。！？、：；""''（）【】\n]/.test(prevChar) &&
                              /[\s，。！？、：；""''（）【】\n]/.test(nextChar);

        if (!isOverlapping(index, end) && (isWordBoundary || name.length >= 2)) {
          mentions.push({
            start: index,
            end: end,
            color: entityColors[entity.type],
            entityId: entity.id,
            entityType: entity.type,
            entityName: entity.name,
            evidenceId: `entity-${entity.id}`,
          });
          usedRanges.push({ start: index, end });
        }

        searchPos = end;
      }
    }
  }

  return mentions.sort((a, b) => a.start - b.start);
};

// Handle entity click
const handleEntityClick = (entityId: string) => {
  const entity = entities.value.find(e => `entity-${e.id}` === entityId);
  if (entity) {
    selectedEntity.value = entity;
  }
};

// Watch for book changes
watch(() => bookStore.currentBook?.id, () => {
  if (showEntityHighlights.value) {
    loadEntities();
  }
});

// Load entities when highlighting is enabled
watch(showEntityHighlights, (enabled) => {
  if (enabled && entities.value.length === 0) {
    loadEntities();
  }
});

const paragraphs = computed(() => {
  if (!bookStore.currentChapter?.content) return [];

  // Split content by double newlines or single newlines
  return bookStore.currentChapter.content
    .split(/\n\s*\n|\n/)
    .map(p => p.trim())
    .filter(p => p.length > 0);
});

// Compute highlight ranges for each paragraph based on active evidence and entities
const paragraphHighlights = computed(() => {
  const highlights: Map<number, HighlightRange[]> = new Map();

  if (!paragraphs.value.length) {
    return highlights;
  }

  // First, add entity highlights for each paragraph
  if (showEntityHighlights.value) {
    paragraphs.value.forEach((para, index) => {
      const entityMentions = findEntityMentions(para);
      if (entityMentions.length > 0) {
        highlights.set(index, entityMentions);
      }
    });
  }

  // Then, add evidence highlights (these take priority with yellow color)
  if (analysisStore.activeEvidence) {
    const excerpt = analysisStore.activeEvidence.excerpt;

    paragraphs.value.forEach((para, index) => {
      const existingHighlights = highlights.get(index) || [];
      const startPos = para.indexOf(excerpt);

      if (startPos !== -1) {
        // Filter out entity highlights that overlap with evidence
        const filtered = existingHighlights.filter(h =>
          h.end <= startPos || h.start >= startPos + excerpt.length
        );
        filtered.push({
          start: startPos,
          end: startPos + excerpt.length,
          evidenceId: `${analysisStore.activeEvidence?.cardId}-${analysisStore.activeEvidence?.evidenceIndex}`,
        });
        highlights.set(index, filtered.sort((a, b) => a.start - b.start));
      } else {
        // Try partial match with first 30 chars
        const partialExcerpt = excerpt.slice(0, 30);
        const partialStart = para.indexOf(partialExcerpt);
        if (partialStart !== -1) {
          const endPos = Math.min(partialStart + excerpt.length, para.length);
          const filtered = existingHighlights.filter(h =>
            h.end <= partialStart || h.start >= endPos
          );
          filtered.push({
            start: partialStart,
            end: endPos,
            evidenceId: `${analysisStore.activeEvidence?.cardId}-${analysisStore.activeEvidence?.evidenceIndex}`,
          });
          highlights.set(index, filtered.sort((a, b) => a.start - b.start));
        }
      }
    });
  }

  return highlights;
});

// Watch for active evidence changes and scroll to highlighted text
watch(
  () => analysisStore.activeEvidence,
  async (newEvidence) => {
    if (!newEvidence) return;

    await nextTick();

    // Find the paragraph with the highlight
    const highlightedParagraphIndex = Array.from(paragraphHighlights.value.keys())[0];
    if (highlightedParagraphIndex !== undefined && readerContent.value) {
      const paragraphElements = readerContent.value.querySelectorAll('.reader-paragraph');
      const targetElement = paragraphElements[highlightedParagraphIndex] as HTMLElement;
      if (targetElement) {
        targetElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }
  }
);

function formatCharCount(count: number): string {
  if (count >= 10000) {
    return `${(count / 10000).toFixed(1)}万字`;
  }
  return `${count}字`;
}

function handleHighlightClick(evidenceId: string) {
  // Check if this is an entity highlight
  if (evidenceId.startsWith('entity-')) {
    handleEntityClick(evidenceId);
    return;
  }

  // Parse the evidence ID and open modal
  const [cardId, indexStr] = evidenceId.split('-');
  const evidenceIndex = parseInt(indexStr, 10);

  // Find the card and its evidence
  const card = analysisStore.techniqueCards.find(c => c.id === cardId)
    || analysisStore.knowledgeCards.find(c => c.id === cardId);

  if (card && card.evidence[evidenceIndex]) {
    analysisStore.openEvidenceModal(cardId, evidenceIndex, card.evidence[evidenceIndex]);
  }
}

// Get entity type label
function getEntityTypeLabel(type: string): string {
  switch (type) {
    case 'character': return '人物';
    case 'setting': return '场景';
    case 'event': return '事件';
    default: return type;
  }
}
</script>

<template>
  <div class="chapter-reader h-full flex flex-col relative">
    <!-- Chapter Header -->
    <header
      v-if="bookStore.currentChapter"
      class="shrink-0 px-8 py-6 border-b border-fabric-sand/30 bg-fabric-warm"
    >
      <div class="max-w-3xl mx-auto flex items-start justify-between">
        <div>
          <h1 class="text-2xl font-bold text-fabric-sepia">
            {{ bookStore.chapterTitle }}
          </h1>
          <div class="mt-2 flex items-center gap-4 text-sm text-fabric-thread">
            <span class="flex items-center gap-1">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
              </svg>
              第 {{ bookStore.currentChapter.index }} 章
            </span>
            <span class="flex items-center gap-1">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              {{ formatCharCount(bookStore.currentChapter.char_count) }}
            </span>
            <span
              v-if="bookStore.currentChapter.analyzed"
              class="flex items-center gap-1 text-green-600 dark:text-green-400"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span>已分析</span>
            </span>
          </div>
        </div>

        <!-- Entity Highlight Toggle -->
        <button
          @click="showEntityHighlights = !showEntityHighlights"
          :class="[
            'flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm transition-all',
            showEntityHighlights
              ? 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300'
              : 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600'
          ]"
          :disabled="loadingEntities"
        >
          <svg v-if="loadingEntities" class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A2 2 0 013 12V7a4 4 0 014-4z" />
          </svg>
          <span>{{ showEntityHighlights ? '隐藏实体' : '显示实体' }}</span>
        </button>
      </div>
    </header>

    <!-- Loading State -->
    <div 
      v-if="bookStore.isLoadingChapter" 
      class="flex-1 flex items-center justify-center"
    >
      <div class="text-center text-gray-500">
        <div class="animate-pulse text-lg mb-2">加载中...</div>
        <div class="text-sm">正在加载章节内容</div>
      </div>
    </div>

    <!-- Empty State -->
    <div 
      v-else-if="!bookStore.currentChapter" 
      class="flex-1 flex items-center justify-center"
    >
      <div class="text-center text-gray-400">
        <div class="text-4xl mb-4">📖</div>
        <div class="text-lg">请从左侧选择章节</div>
      </div>
    </div>

    <!-- Content -->
    <article
      v-else
      ref="readerContent"
      class="flex-1 overflow-y-auto px-8 py-8"
    >
      <div class="max-w-3xl mx-auto">
        <div class="reader-content space-y-4">
          <p
            v-for="(paragraph, index) in paragraphs"
            :key="index"
            class="reader-paragraph text-indent-2"
          >
            <TextHighlight
              v-if="paragraphHighlights.has(index)"
              :text="paragraph"
              :highlights="paragraphHighlights.get(index)"
              :active-highlight-id="analysisStore.activeEvidence ? `${analysisStore.activeEvidence.cardId}-${analysisStore.activeEvidence.evidenceIndex}` : null"
              @highlight-click="handleHighlightClick"
            />
            <template v-else>{{ paragraph }}</template>
          </p>
        </div>
      </div>
    </article>

    <!-- Navigation Footer -->
    <footer
      v-if="bookStore.currentChapter"
      class="shrink-0 px-8 py-4 border-t border-fabric-sand/30 bg-fabric-warm"
    >
      <div class="max-w-3xl mx-auto flex items-center justify-between">
        <button
          @click="bookStore.goToPrevChapter()"
          :disabled="!bookStore.hasPrevChapter"
          :class="[
            'fabric-btn flex items-center gap-2 min-w-[120px] justify-center',
            !bookStore.hasPrevChapter && 'opacity-40 cursor-not-allowed hover:bg-fabric-canvas hover:shadow-none hover:translate-y-0'
          ]"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
          <span>上一章</span>
        </button>

        <div class="flex items-center gap-3 text-sm text-fabric-thread">
          <span class="font-medium">{{ bookStore.currentChapterIndex }}</span>
          <span class="text-fabric-sand">/</span>
          <span>{{ bookStore.totalChapters }}</span>
        </div>

        <button
          @click="bookStore.goToNextChapter()"
          :disabled="!bookStore.hasNextChapter"
          :class="[
            'fabric-btn flex items-center gap-2 min-w-[120px] justify-center',
            !bookStore.hasNextChapter && 'opacity-40 cursor-not-allowed hover:bg-fabric-canvas hover:shadow-none hover:translate-y-0'
          ]"
        >
          <span>下一章</span>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
        </button>
      </div>
    </footer>

    <!-- Evidence Modal -->
    <EvidenceModal />

    <!-- Entity Legend (when highlighting is enabled) -->
    <div
      v-if="showEntityHighlights && bookStore.currentChapter"
      class="absolute bottom-20 right-4 bg-white/95 dark:bg-gray-800/95 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 p-3 text-xs z-10"
    >
      <p class="font-medium text-gray-700 dark:text-gray-300 mb-2">实体图例</p>
      <div class="space-y-1.5">
        <div class="flex items-center gap-2">
          <div class="w-4 h-3 rounded" style="background-color: rgba(139, 92, 246, 0.25); border: 1px solid #8b5cf6;"></div>
          <span class="text-gray-600 dark:text-gray-400">人物</span>
        </div>
        <div class="flex items-center gap-2">
          <div class="w-4 h-3 rounded" style="background-color: rgba(59, 130, 246, 0.25); border: 1px solid #3b82f6;"></div>
          <span class="text-gray-600 dark:text-gray-400">场景</span>
        </div>
        <div class="flex items-center gap-2">
          <div class="w-4 h-3 rounded" style="background-color: rgba(249, 115, 22, 0.25); border: 1px solid #f97316;"></div>
          <span class="text-gray-600 dark:text-gray-400">事件</span>
        </div>
      </div>
      <p class="mt-2 text-gray-400 dark:text-gray-500 text-[10px]">点击高亮查看详情</p>
    </div>

    <!-- Entity Detail Panel -->
    <Teleport to="body">
      <div
        v-if="selectedEntity"
        class="fixed inset-0 z-50 flex items-center justify-center"
        @click.self="selectedEntity = null"
      >
        <div class="absolute inset-0 bg-black/30 backdrop-blur-sm"></div>
        <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-md w-full mx-4 overflow-hidden">
          <!-- Header -->
          <div
            class="px-6 py-4 border-b border-gray-200 dark:border-gray-700"
            :style="{ backgroundColor: entityColors[selectedEntity.type] }"
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div
                  class="w-10 h-10 rounded-full flex items-center justify-center text-white font-bold text-lg"
                  :style="{ backgroundColor: entityBorderColors[selectedEntity.type] }"
                >
                  {{ selectedEntity.name.charAt(0) }}
                </div>
                <div>
                  <h3 class="font-bold text-gray-900 dark:text-white text-lg">{{ selectedEntity.name }}</h3>
                  <span
                    class="inline-block px-2 py-0.5 rounded-full text-xs font-medium"
                    :style="{ backgroundColor: entityBorderColors[selectedEntity.type], color: 'white' }"
                  >
                    {{ getEntityTypeLabel(selectedEntity.type) }}
                  </span>
                </div>
              </div>
              <button
                @click="selectedEntity = null"
                class="p-1.5 rounded-lg hover:bg-black/10 dark:hover:bg-white/10 transition-colors"
              >
                <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>

          <!-- Content -->
          <div class="px-6 py-4 max-h-80 overflow-y-auto">
            <!-- Aliases -->
            <div v-if="selectedEntity.aliases && selectedEntity.aliases.length > 0" class="mb-4">
              <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">别名</p>
              <div class="flex flex-wrap gap-1">
                <span
                  v-for="alias in selectedEntity.aliases"
                  :key="alias"
                  class="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-sm text-gray-700 dark:text-gray-300"
                >
                  {{ alias }}
                </span>
              </div>
            </div>

            <!-- Description -->
            <div v-if="selectedEntity.description" class="mb-4">
              <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">描述</p>
              <p class="text-sm text-gray-700 dark:text-gray-300 leading-relaxed">
                {{ selectedEntity.description }}
              </p>
            </div>

            <!-- Character-specific info -->
            <template v-if="selectedEntity.type === 'character'">
              <div v-if="(selectedEntity as CharacterEntity).role" class="mb-4">
                <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">角色类型</p>
                <p class="text-sm text-gray-700 dark:text-gray-300">{{ (selectedEntity as CharacterEntity).role }}</p>
              </div>
              <div v-if="(selectedEntity as CharacterEntity).traits && (selectedEntity as CharacterEntity).traits.length > 0" class="mb-4">
                <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">特征</p>
                <div class="flex flex-wrap gap-1">
                  <span
                    v-for="trait in (selectedEntity as CharacterEntity).traits"
                    :key="trait"
                    class="px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 rounded text-sm text-purple-700 dark:text-purple-300"
                  >
                    {{ trait }}
                  </span>
                </div>
              </div>
            </template>

            <!-- Setting-specific info -->
            <template v-if="selectedEntity.type === 'setting'">
              <div v-if="(selectedEntity as SettingEntity).setting_type" class="mb-4">
                <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">场景类型</p>
                <p class="text-sm text-gray-700 dark:text-gray-300">{{ (selectedEntity as SettingEntity).setting_type }}</p>
              </div>
            </template>

            <!-- Event-specific info -->
            <template v-if="selectedEntity.type === 'event'">
              <div v-if="(selectedEntity as EventEntity).event_type" class="mb-4">
                <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">事件类型</p>
                <p class="text-sm text-gray-700 dark:text-gray-300">{{ (selectedEntity as EventEntity).event_type }}</p>
              </div>
            </template>
          </div>

          <!-- Footer -->
          <div class="px-6 py-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
            <button
              @click="selectedEntity = null"
              class="w-full py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors"
            >
              关闭
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.text-indent-2 {
  text-indent: 2em;
}
</style>
