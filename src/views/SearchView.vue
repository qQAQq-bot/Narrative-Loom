<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useTauri } from '@/composables/useTauri';
import { useBookStore } from '@/stores/book';

const route = useRoute();
const router = useRouter();
const { invoke } = useTauri();
const bookStore = useBookStore();

const bookId = computed(() => route.params.id as string);
const searchInputRef = ref<HTMLInputElement | null>(null);

// Search state
const searchQuery = ref('');
const isLoading = ref(false);
const results = ref<SearchResult[]>([]);
const semanticResults = ref<SemanticSearchResult[]>([]);
const error = ref<string | null>(null);

// Search mode
const searchMode = ref<'classic' | 'semantic'>('classic');
const semanticSearchMode = ref<'hybrid' | 'vector' | 'keyword'>('hybrid');

// Filter state
const searchChapters = ref(true);
const searchCharacters = ref(true);
const searchSettings = ref(true);
const searchEvents = ref(true);

interface SearchResult {
  result_type: string;
  id: string;
  title: string;
  content: string;
  chapter_id: string | null;
  chapter_title: string | null;
  highlights: { start: number; end: number }[];
  score: number;
}

interface SemanticSearchResult {
  chunk_id: string;
  content: string;
  chapter_id: string;
  chunk_type: string;
  score: number;
  search_mode: string;
  vector_rank: number | null;
  keyword_rank: number | null;
  char_start: number;
  char_end: number;
  entities_mentioned: string[];
}

interface SearchOptions {
  query: string;
  search_chapters: boolean;
  search_characters: boolean;
  search_settings: boolean;
  search_events: boolean;
  limit: number | null;
}

// Computed
const groupedResults = computed(() => {
  const groups: Record<string, SearchResult[]> = {
    chapter: [],
    character: [],
    setting: [],
    event: [],
  };

  for (const result of results.value) {
    if (groups[result.result_type]) {
      groups[result.result_type].push(result);
    }
  }

  return groups;
});

const resultCounts = computed(() => ({
  chapter: groupedResults.value.chapter.length,
  character: groupedResults.value.character.length,
  setting: groupedResults.value.setting.length,
  event: groupedResults.value.event.length,
  total: results.value.length,
}));

const hasResults = computed(() => {
  if (searchMode.value === 'semantic') {
    return semanticResults.value.length > 0;
  }
  return results.value.length > 0;
});
const hasSearched = ref(false);

// Actions
async function performSearch() {
  if (!searchQuery.value.trim() || !bookId.value) return;

  isLoading.value = true;
  error.value = null;
  hasSearched.value = true;

  try {
    if (searchMode.value === 'semantic') {
      semanticResults.value = await invoke<SemanticSearchResult[]>('semantic_search', {
        bookId: bookId.value,
        query: searchQuery.value.trim(),
        topK: 20,
        searchMode: semanticSearchMode.value,
        chunkType: null,
        excludeChapterId: null,
      });
      results.value = [];
    } else {
      const options: SearchOptions = {
        query: searchQuery.value.trim(),
        search_chapters: searchChapters.value,
        search_characters: searchCharacters.value,
        search_settings: searchSettings.value,
        search_events: searchEvents.value,
        limit: 100,
      };

      results.value = await invoke<SearchResult[]>('search', {
        bookId: bookId.value,
        options,
      });
      semanticResults.value = [];
    }
  } catch (e) {
    const message = typeof e === 'string' ? e : (e instanceof Error ? e.message : '未知错误');
    error.value = `搜索失败: ${message}`;
    console.error('Search failed:', e);
  } finally {
    isLoading.value = false;
  }
}

function navigateToResult(result: SearchResult) {
  if (result.result_type === 'chapter' && result.chapter_id) {
    bookStore.loadChapter(result.chapter_id);
    router.push(`/book/${bookId.value}`);
  } else if (result.result_type === 'character') {
    router.push(`/book/${bookId.value}/bible?tab=characters&id=${result.id}`);
  } else if (result.result_type === 'setting') {
    router.push(`/book/${bookId.value}/bible?tab=settings&id=${result.id}`);
  } else if (result.result_type === 'event') {
    router.push(`/book/${bookId.value}/bible?tab=events&id=${result.id}`);
  }
}

function navigateToSemanticResult(result: SemanticSearchResult) {
  bookStore.loadChapter(result.chapter_id);
  router.push(`/book/${bookId.value}`);
}

function getChunkTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    paragraph: '段落',
    entity: '实体',
    chapter: '章节',
  };
  return labels[type] || type;
}

function getChunkTypeColor(type: string): string {
  const colors: Record<string, string> = {
    paragraph: 'bg-accent-technique/15 text-accent-technique',
    entity: 'bg-accent-setting/15 text-accent-setting',
    chapter: 'bg-accent-character/15 text-accent-character',
  };
  return colors[type] || 'bg-fabric-canvas/80 text-fabric-thread';
}

function getSearchModeLabel(mode: string): string {
  const labels: Record<string, string> = {
    hybrid: '混合',
    vector: '向量',
    keyword: '关键词',
  };
  return labels[mode] || mode;
}

function getResultTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    chapter: '章节',
    character: '人物',
    setting: '设定',
    event: '事件',
  };
  return labels[type] || type;
}

function getResultTypeColor(type: string): string {
  const colors: Record<string, string> = {
    chapter: 'bg-accent-technique/15 text-accent-technique',
    character: 'bg-accent-character/15 text-accent-character',
    setting: 'bg-accent-setting/15 text-accent-setting',
    event: 'bg-accent-event/15 text-accent-event',
  };
  return colors[type] || 'bg-fabric-canvas/80 text-fabric-thread';
}

function getResultBorderColor(type: string): string {
  const colors: Record<string, string> = {
    chapter: 'border-l-accent-technique',
    character: 'border-l-accent-character',
    setting: 'border-l-accent-setting',
    event: 'border-l-accent-event',
  };
  return colors[type] || '';
}

function getResultIcon(type: string): string {
  const icons: Record<string, string> = {
    chapter: 'M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253',
    character: 'M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z',
    setting: 'M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z M15 11a3 3 0 11-6 0 3 3 0 016 0z',
    event: 'M13 10V3L4 14h7v7l9-11h-7z',
  };
  return icons[type] || '';
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    performSearch();
  }
}

onMounted(async () => {
  if (bookId.value) {
    bookStore.loadBook(bookId.value);
  }

  // Read query parameter from SearchBar navigation
  const queryParam = route.query.q as string | undefined;
  if (queryParam?.trim()) {
    searchQuery.value = queryParam.trim();
    await nextTick();
    performSearch();
  } else {
    // Focus the search input when no query
    await nextTick();
    searchInputRef.value?.focus();
  }
});

watch(() => route.params.id, () => {
  if (bookId.value) {
    bookStore.loadBook(bookId.value);
    searchQuery.value = '';
    results.value = [];
    semanticResults.value = [];
    hasSearched.value = false;
    error.value = null;
  }
});
</script>

<template>
  <div class="h-full flex flex-col bg-fabric-cream overflow-hidden">
    <!-- Header -->
    <header class="bg-fabric-warm border-b border-fabric-sand/40 px-6 py-4 shrink-0">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h1 class="text-xl font-bold text-fabric-sepia">搜索</h1>
          <p class="text-sm text-fabric-thread/60 mt-0.5">
            {{ bookStore.currentBook?.title || '加载中...' }}
          </p>
        </div>
        <router-link
          :to="`/book/${bookId}`"
          class="flex items-center gap-2 px-4 py-2 text-fabric-thread hover:text-fabric-sepia hover:bg-fabric-canvas/50 rounded-lg transition-colors"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M9.707 16.707a1 1 0 01-1.414 0l-6-6a1 1 0 010-1.414l6-6a1 1 0 011.414 1.414L5.414 9H17a1 1 0 110 2H5.414l4.293 4.293a1 1 0 010 1.414z" clip-rule="evenodd" />
          </svg>
          <span>返回阅读</span>
        </router-link>
      </div>

      <!-- Search Input -->
      <div class="flex gap-3">
        <div class="flex-1 relative">
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            type="text"
            placeholder="输入关键词搜索..."
            class="fabric-input pl-10"
            @keydown="handleKeydown"
          />
          <svg
            class="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-fabric-thread/40"
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 20 20"
            fill="currentColor"
          >
            <path fill-rule="evenodd" d="M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z" clip-rule="evenodd" />
          </svg>
        </div>
        <button
          @click="performSearch"
          :disabled="isLoading || !searchQuery.trim()"
          class="fabric-btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <span v-if="isLoading">搜索中...</span>
          <span v-else>搜索</span>
        </button>
      </div>

      <!-- Search Mode Selector -->
      <div class="mt-4 flex items-center gap-6">
        <div class="flex items-center gap-3">
          <span class="text-sm font-medium text-fabric-thread">搜索模式:</span>
          <div class="flex rounded-lg overflow-hidden border border-fabric-sand/50">
            <button
              @click="searchMode = 'classic'"
              :class="[
                'px-3 py-1.5 text-sm font-medium transition-colors',
                searchMode === 'classic'
                  ? 'bg-primary-500 text-white'
                  : 'bg-fabric-warm text-fabric-thread hover:bg-fabric-canvas/50'
              ]"
            >
              经典搜索
            </button>
            <button
              @click="searchMode = 'semantic'"
              :class="[
                'px-3 py-1.5 text-sm font-medium transition-colors border-l border-fabric-sand/50',
                searchMode === 'semantic'
                  ? 'bg-primary-500 text-white'
                  : 'bg-fabric-warm text-fabric-thread hover:bg-fabric-canvas/50'
              ]"
            >
              语义搜索
            </button>
          </div>
        </div>

        <!-- Semantic Search Mode Options -->
        <div v-if="searchMode === 'semantic'" class="flex items-center gap-3">
          <span class="text-sm text-fabric-thread/40">|</span>
          <span class="text-sm font-medium text-fabric-thread">算法:</span>
          <select
            v-model="semanticSearchMode"
            class="fabric-select text-sm"
          >
            <option value="hybrid">混合 (推荐)</option>
            <option value="vector">向量</option>
            <option value="keyword">关键词</option>
          </select>
        </div>
      </div>

      <!-- Classic Filters -->
      <div v-if="searchMode === 'classic'" class="mt-4 flex flex-wrap gap-4">
        <label class="flex items-center gap-2 cursor-pointer group">
          <input
            v-model="searchChapters"
            type="checkbox"
            class="w-4 h-4 text-primary-500 border-fabric-sand/50 rounded focus:ring-primary-400/50"
          />
          <span class="text-sm text-fabric-thread group-hover:text-fabric-sepia transition-colors">章节内容</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer group">
          <input
            v-model="searchCharacters"
            type="checkbox"
            class="w-4 h-4 text-primary-500 border-fabric-sand/50 rounded focus:ring-primary-400/50"
          />
          <span class="text-sm text-fabric-thread group-hover:text-fabric-sepia transition-colors">人物</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer group">
          <input
            v-model="searchSettings"
            type="checkbox"
            class="w-4 h-4 text-primary-500 border-fabric-sand/50 rounded focus:ring-primary-400/50"
          />
          <span class="text-sm text-fabric-thread group-hover:text-fabric-sepia transition-colors">设定</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer group">
          <input
            v-model="searchEvents"
            type="checkbox"
            class="w-4 h-4 text-primary-500 border-fabric-sand/50 rounded focus:ring-primary-400/50"
          />
          <span class="text-sm text-fabric-thread group-hover:text-fabric-sepia transition-colors">事件</span>
        </label>
      </div>
    </header>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      <!-- Loading -->
      <div v-if="isLoading" class="flex flex-col items-center justify-center h-64 text-fabric-thread/50">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500 mb-3"></div>
        <span>搜索中...</span>
      </div>

      <!-- Error -->
      <div v-else-if="error" class="flex flex-col items-center justify-center h-64">
        <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800/40 rounded-xl px-6 py-4 max-w-md text-center">
          <svg class="w-8 h-8 text-red-400 mx-auto mb-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
          </svg>
          <p class="text-sm text-red-600 dark:text-red-400">{{ error }}</p>
        </div>
      </div>

      <!-- Empty State - Before Search -->
      <div
        v-else-if="!hasSearched"
        class="flex flex-col items-center justify-center h-64 text-fabric-thread/40"
      >
        <svg class="w-16 h-16 mb-4 text-fabric-sand/60" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <h3 class="text-lg font-medium text-fabric-thread/60 mb-2">搜索书籍内容</h3>
        <p class="text-sm text-fabric-thread/40 max-w-md text-center">
          输入关键词搜索章节内容、人物、设定、事件等信息
        </p>
      </div>

      <!-- Empty State - No Results -->
      <div
        v-else-if="!hasResults"
        class="flex flex-col items-center justify-center h-64 text-fabric-thread/40"
      >
        <svg class="w-16 h-16 mb-4 text-fabric-sand/60" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M12 12h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <h3 class="text-lg font-medium text-fabric-thread/60 mb-2">没有找到结果</h3>
        <p class="text-sm text-fabric-thread/40">
          尝试使用不同的关键词或调整筛选条件
        </p>
      </div>

      <!-- Results -->
      <div v-else class="space-y-6">
        <!-- Classic Search Results -->
        <template v-if="searchMode === 'classic'">
          <!-- Results Summary -->
          <div class="flex items-center gap-3 text-sm text-fabric-thread/60">
            <span>找到 {{ resultCounts.total }} 个结果</span>
            <span v-if="resultCounts.chapter > 0" class="px-2 py-0.5 bg-accent-technique/10 text-accent-technique rounded">
              {{ resultCounts.chapter }} 章节
            </span>
            <span v-if="resultCounts.character > 0" class="px-2 py-0.5 bg-accent-character/10 text-accent-character rounded">
              {{ resultCounts.character }} 人物
            </span>
            <span v-if="resultCounts.setting > 0" class="px-2 py-0.5 bg-accent-setting/10 text-accent-setting rounded">
              {{ resultCounts.setting }} 设定
            </span>
            <span v-if="resultCounts.event > 0" class="px-2 py-0.5 bg-accent-event/10 text-accent-event rounded">
              {{ resultCounts.event }} 事件
            </span>
          </div>

          <!-- Result Groups -->
          <template v-for="(items, type) in groupedResults" :key="type">
            <div v-if="items.length > 0" class="space-y-3">
              <h2 class="text-sm font-semibold text-fabric-thread/50 uppercase tracking-wider flex items-center gap-2">
                <svg class="w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" :d="getResultIcon(type as string)" />
                </svg>
                {{ getResultTypeLabel(type as string) }}
                <span class="text-fabric-thread/30 font-normal">({{ items.length }})</span>
              </h2>

              <div class="space-y-2">
                <div
                  v-for="result in items"
                  :key="result.id"
                  @click="navigateToResult(result)"
                  :class="[
                    'bg-fabric-warm border border-fabric-sand/30 rounded-xl p-4 cursor-pointer',
                    'hover:border-primary-400/50 transition-all duration-200',
                    'border-l-[3px] overflow-hidden',
                    getResultBorderColor(result.result_type),
                  ]"
                >
                  <div class="flex items-start gap-3">
                    <span :class="['shrink-0 px-2 py-1 text-xs rounded-full font-medium', getResultTypeColor(result.result_type)]">
                      {{ getResultTypeLabel(result.result_type) }}
                    </span>
                    <div class="flex-1 min-w-0">
                      <h3 class="font-medium text-fabric-sepia mb-1">{{ result.title }}</h3>
                      <p class="text-sm text-fabric-thread/70 line-clamp-2">{{ result.content }}</p>
                    </div>
                    <svg class="w-5 h-5 text-fabric-thread/30 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                      <path fill-rule="evenodd" d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z" clip-rule="evenodd" />
                    </svg>
                  </div>
                </div>
              </div>
            </div>
          </template>
        </template>

        <!-- Semantic Search Results -->
        <template v-else>
          <!-- Results Summary -->
          <div class="flex items-center gap-3 text-sm text-fabric-thread/60">
            <span>找到 {{ semanticResults.length }} 个相关段落</span>
            <span class="px-2 py-0.5 bg-primary-500/10 text-primary-700 dark:text-primary-300 rounded">
              {{ getSearchModeLabel(semanticSearchMode) }}搜索
            </span>
          </div>

          <!-- Semantic Result List -->
          <div class="space-y-3">
            <div
              v-for="(result, index) in semanticResults"
              :key="result.chunk_id"
              @click="navigateToSemanticResult(result)"
              class="bg-fabric-warm border border-fabric-sand/30 rounded-xl p-4 cursor-pointer hover:border-primary-400/50 transition-all duration-200"
            >
              <div class="flex items-start gap-3">
                <!-- Rank Badge -->
                <div class="shrink-0 flex flex-col items-center">
                  <span class="w-8 h-8 flex items-center justify-center bg-fabric-canvas/80 text-fabric-thread rounded-full text-sm font-medium">
                    {{ index + 1 }}
                  </span>
                </div>

                <div class="flex-1 min-w-0">
                  <!-- Meta Info -->
                  <div class="flex items-center gap-2 mb-2">
                    <span :class="['px-2 py-0.5 text-xs rounded-full font-medium', getChunkTypeColor(result.chunk_type)]">
                      {{ getChunkTypeLabel(result.chunk_type) }}
                    </span>
                    <span class="text-xs text-fabric-thread/40">
                      相似度: {{ (result.score * 100).toFixed(1) }}%
                    </span>
                    <template v-if="result.vector_rank !== null || result.keyword_rank !== null">
                      <span class="text-xs text-fabric-thread/20">|</span>
                      <span v-if="result.vector_rank !== null" class="text-xs text-accent-technique">
                        向量#{{ result.vector_rank + 1 }}
                      </span>
                      <span v-if="result.keyword_rank !== null" class="text-xs text-accent-character">
                        关键词#{{ result.keyword_rank + 1 }}
                      </span>
                    </template>
                  </div>

                  <!-- Content -->
                  <p class="text-sm text-fabric-thread/80 line-clamp-3">{{ result.content }}</p>

                  <!-- Entities -->
                  <div v-if="result.entities_mentioned.length > 0" class="mt-2 flex flex-wrap gap-1">
                    <span
                      v-for="entity in result.entities_mentioned.slice(0, 5)"
                      :key="entity"
                      class="px-1.5 py-0.5 text-xs bg-fabric-canvas/80 text-fabric-thread border border-fabric-sand/30 rounded"
                    >
                      {{ entity }}
                    </span>
                    <span v-if="result.entities_mentioned.length > 5" class="text-xs text-fabric-thread/40">
                      +{{ result.entities_mentioned.length - 5 }}
                    </span>
                  </div>
                </div>

                <svg class="w-5 h-5 text-fabric-thread/30 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                  <path fill-rule="evenodd" d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z" clip-rule="evenodd" />
                </svg>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
