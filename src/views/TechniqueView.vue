<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useTauri } from '@/composables/useTauri';
import { useBookStore } from '@/stores/book';

// Types for the new technique library
interface TechniqueType {
  id: string;
  name: string;
  category: string;
  description: string | null;
  principle: string | null;
  example_count: number;
  created_at: string;
  updated_at: string;
}

interface TechniqueExample {
  id: string;
  technique_type_id: string;
  chapter_id: string;
  title: string;
  description: string;
  mechanism: string | null;
  evidence: string[];
  is_featured: boolean;
  created_at: string;
}

interface TechniqueExampleWithChapter extends TechniqueExample {
  chapter_title: string | null;
  chapter_index: number;
}

interface TechniqueTypeWithExamples extends TechniqueType {
  examples: TechniqueExampleWithChapter[];
}

interface TechniqueLibraryStats {
  type_count: number;
  example_count: number;
  featured_count: number;
}

const route = useRoute();
const router = useRouter();
const { invoke } = useTauri();
const bookStore = useBookStore();

const bookId = computed(() => route.params.id as string);

// State
const techniqueLibrary = ref<TechniqueTypeWithExamples[]>([]);
const stats = ref<TechniqueLibraryStats>({ type_count: 0, example_count: 0, featured_count: 0 });
const isLoading = ref(false);
const error = ref<string | null>(null);
const selectedCategory = ref<string | null>(null);
const expandedTypes = ref<Set<string>>(new Set());
const selectedExample = ref<TechniqueExampleWithChapter | null>(null);
const selectedType = ref<TechniqueTypeWithExamples | null>(null);

// Category labels
const categoryLabels: Record<string, string> = {
  '叙事': '叙事技法',
  '对话': '对话技法',
  '描写': '描写技法',
  '结构': '结构技法',
  '节奏': '节奏技法',
  '张力': '张力技法',
  '伏笔': '伏笔技法',
  '人物': '人物刻画',
  '氛围': '氛围营造',
  '场景': '场景技法',
  '悬念': '悬念技法',
  '主题': '主题技法',
  '叙述': '叙述声音',
  '其他': '其他技法',
};

// Category colors
const categoryColors: Record<string, { bg: string; text: string; border: string }> = {
  '叙事': { bg: 'bg-blue-50', text: 'text-blue-700', border: 'border-l-blue-500' },
  '对话': { bg: 'bg-green-50', text: 'text-green-700', border: 'border-l-green-500' },
  '描写': { bg: 'bg-purple-50', text: 'text-purple-700', border: 'border-l-purple-500' },
  '结构': { bg: 'bg-orange-50', text: 'text-orange-700', border: 'border-l-orange-500' },
  '节奏': { bg: 'bg-pink-50', text: 'text-pink-700', border: 'border-l-pink-500' },
  '张力': { bg: 'bg-red-50', text: 'text-red-700', border: 'border-l-red-500' },
  '伏笔': { bg: 'bg-amber-50', text: 'text-amber-700', border: 'border-l-amber-500' },
  '人物': { bg: 'bg-cyan-50', text: 'text-cyan-700', border: 'border-l-cyan-500' },
  '氛围': { bg: 'bg-indigo-50', text: 'text-indigo-700', border: 'border-l-indigo-500' },
  '场景': { bg: 'bg-teal-50', text: 'text-teal-700', border: 'border-l-teal-500' },
  '悬念': { bg: 'bg-rose-50', text: 'text-rose-700', border: 'border-l-rose-500' },
  '主题': { bg: 'bg-violet-50', text: 'text-violet-700', border: 'border-l-violet-500' },
  '叙述': { bg: 'bg-sky-50', text: 'text-sky-700', border: 'border-l-sky-500' },
  '其他': { bg: 'bg-gray-50', text: 'text-gray-700', border: 'border-l-gray-500' },
};

// Computed
const categories = computed(() => {
  const cats = new Set(techniqueLibrary.value.map(t => t.category));
  return Array.from(cats).sort();
});

const filteredLibrary = computed(() => {
  if (!selectedCategory.value) {
    return techniqueLibrary.value;
  }
  return techniqueLibrary.value.filter(t => t.category === selectedCategory.value);
});

const groupedByCategory = computed(() => {
  const grouped: Record<string, TechniqueTypeWithExamples[]> = {};
  for (const tt of filteredLibrary.value) {
    if (!grouped[tt.category]) {
      grouped[tt.category] = [];
    }
    grouped[tt.category].push(tt);
  }
  return grouped;
});

// Actions
async function loadLibrary() {
  if (!bookId.value) return;

  isLoading.value = true;
  error.value = null;

  try {
    const [library, libraryStats] = await Promise.all([
      invoke<TechniqueTypeWithExamples[]>('get_technique_library', { bookId: bookId.value }),
      invoke<TechniqueLibraryStats>('get_technique_library_stats', { bookId: bookId.value }),
    ]);
    techniqueLibrary.value = library;
    stats.value = libraryStats;
  } catch (e) {
    error.value = '加载技法库失败';
    console.error(e);
  } finally {
    isLoading.value = false;
  }
}

function toggleExpand(typeId: string) {
  const newSet = new Set(expandedTypes.value);
  if (newSet.has(typeId)) {
    newSet.delete(typeId);
  } else {
    newSet.add(typeId);
  }
  expandedTypes.value = newSet;
}

function selectExample(example: TechniqueExampleWithChapter, type: TechniqueTypeWithExamples) {
  selectedExample.value = example;
  selectedType.value = type;
}

function closeDetail() {
  selectedExample.value = null;
  selectedType.value = null;
}

async function toggleFeatured(exampleId: string, featured: boolean) {
  try {
    await invoke('toggle_example_featured', {
      bookId: bookId.value,
      exampleId,
      featured,
    });
    // Update local state
    for (const tt of techniqueLibrary.value) {
      const example = tt.examples.find(e => e.id === exampleId);
      if (example) {
        example.is_featured = featured;
        break;
      }
    }
    if (selectedExample.value?.id === exampleId) {
      selectedExample.value.is_featured = featured;
    }
  } catch (e) {
    console.error('更新精选状态失败', e);
  }
}

async function deleteExample(exampleId: string) {
  try {
    await invoke('delete_technique_example', {
      bookId: bookId.value,
      exampleId,
    });
    // Reload library
    await loadLibrary();
    if (selectedExample.value?.id === exampleId) {
      closeDetail();
    }
  } catch (e) {
    console.error('删除案例失败', e);
  }
}

// Clear library state
const showClearConfirm = ref(false);
const isClearing = ref(false);

function confirmClearLibrary() {
  showClearConfirm.value = true;
}

async function executeClearLibrary() {
  isClearing.value = true;
  try {
    await invoke('clear_technique_library', {
      bookId: bookId.value,
    });
    // Clear local state
    techniqueLibrary.value = [];
    stats.value = { type_count: 0, example_count: 0, featured_count: 0 };
    showClearConfirm.value = false;
    closeDetail();
  } catch (e) {
    console.error('清空技法库失败', e);
    error.value = '清空技法库失败';
  } finally {
    isClearing.value = false;
  }
}

function cancelClear() {
  showClearConfirm.value = false;
}

function goToChapter(chapterId: string) {
  router.push(`/book/${bookId.value}`);
  // The chapter loading will be handled by the chapter view
  bookStore.loadChapter(chapterId);
}

function getCategoryColor(category: string) {
  return categoryColors[category] || categoryColors['其他'];
}

function getCategoryLabel(category: string) {
  return categoryLabels[category] || category;
}

onMounted(() => {
  if (bookId.value) {
    bookStore.loadBook(bookId.value);
    loadLibrary();
  }
});

watch(() => route.params.id, () => {
  if (bookId.value) {
    loadLibrary();
  }
});
</script>

<template>
  <div class="h-full flex flex-col bg-gray-50/50 overflow-hidden">
    <!-- Header -->
    <header class="bg-white border-b border-gray-200 px-6 py-4 shrink-0 shadow-sm">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-xl font-bold text-gray-900">技法库</h1>
          <p class="text-sm text-gray-500 mt-0.5">
            {{ bookStore.currentBook?.title || '加载中...' }}
            <span v-if="stats.type_count > 0" class="ml-2">
              · {{ stats.type_count }} 种技法 · {{ stats.example_count }} 个案例
              <span v-if="stats.featured_count > 0" class="text-yellow-600">
                · {{ stats.featured_count }} 个精选
              </span>
            </span>
          </p>
        </div>
        <router-link
          :to="`/book/${bookId}`"
          class="flex items-center gap-2 px-4 py-2 text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M9.707 16.707a1 1 0 01-1.414 0l-6-6a1 1 0 010-1.414l6-6a1 1 0 011.414 1.414L5.414 9H17a1 1 0 110 2H5.414l4.293 4.293a1 1 0 010 1.414z" clip-rule="evenodd" />
          </svg>
          <span>返回阅读</span>
        </router-link>
        <!-- Clear Library Button -->
        <button
          v-if="stats.example_count > 0"
          @click="confirmClearLibrary"
          class="flex items-center gap-2 px-4 py-2 text-red-600 hover:text-red-700 hover:bg-red-50 rounded-lg transition-colors"
        >
          <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          <span>清空技法库</span>
        </button>
      </div>

      <!-- Category Filter -->
      <div v-if="categories.length > 0" class="mt-4 flex flex-wrap gap-2">
        <button
          @click="selectedCategory = null"
          :class="[
            'px-3 py-1.5 text-sm rounded-lg transition-colors',
            selectedCategory === null
              ? 'bg-primary-100 text-primary-700 font-medium'
              : 'bg-gray-100 text-gray-600 hover:bg-gray-200',
          ]"
        >
          全部 ({{ techniqueLibrary.length }})
        </button>
        <button
          v-for="cat in categories"
          :key="cat"
          @click="selectedCategory = selectedCategory === cat ? null : cat"
          :class="[
            'px-3 py-1.5 text-sm rounded-lg transition-colors',
            selectedCategory === cat
              ? getCategoryColor(cat).bg + ' ' + getCategoryColor(cat).text + ' font-medium'
              : 'bg-gray-100 text-gray-600 hover:bg-gray-200',
          ]"
        >
          {{ getCategoryLabel(cat) }}
          ({{ techniqueLibrary.filter(t => t.category === cat).length }})
        </button>
      </div>
    </header>

    <!-- Content -->
    <div class="flex-1 overflow-hidden flex">
      <!-- Main List -->
      <div class="flex-1 overflow-y-auto p-6">
        <!-- Loading -->
        <div v-if="isLoading" class="flex flex-col items-center justify-center h-full text-gray-400">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500 mb-3"></div>
          <span>加载中...</span>
        </div>

        <!-- Error -->
        <div v-else-if="error" class="flex flex-col items-center justify-center h-full text-red-500">
          <span class="text-lg mb-2">!</span>
          <span>{{ error }}</span>
        </div>

        <!-- Empty State -->
        <div
          v-else-if="techniqueLibrary.length === 0"
          class="flex flex-col items-center justify-center h-full text-gray-400"
        >
          <div class="text-6xl mb-4">*</div>
          <h3 class="text-lg font-medium text-gray-600 mb-2">技法库是空的</h3>
          <p class="text-sm text-gray-500 max-w-md text-center">
            在阅读章节时进行分析，技法会自动添加到技法库中。你可以将喜欢的技法案例标记为精选。
          </p>
          <router-link
            :to="`/book/${bookId}`"
            class="mt-6 px-4 py-2 bg-primary-100 text-primary-700 rounded-lg hover:bg-primary-200 transition-colors"
          >
            开始阅读
          </router-link>
        </div>

        <!-- Technique Library List -->
        <div v-else class="space-y-4">
          <div v-for="(types, category) in groupedByCategory" :key="category" class="space-y-3">
            <h2 class="text-sm font-semibold text-gray-500 uppercase tracking-wider flex items-center gap-2">
              <span :class="['w-3 h-3 rounded', getCategoryColor(category as string).bg]"></span>
              {{ getCategoryLabel(category as string) }}
              <span class="text-gray-400 font-normal">({{ types.length }})</span>
            </h2>

            <!-- Technique Types in this category -->
            <div class="space-y-2">
              <div
                v-for="tt in types"
                :key="tt.id"
                class="bg-white rounded-xl border border-gray-100 overflow-hidden"
              >
                <!-- Type Header -->
                <button
                  @click="toggleExpand(tt.id)"
                  :class="[
                    'w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 transition-colors border-l-4',
                    getCategoryColor(tt.category).border,
                  ]"
                >
                  <div class="flex items-center gap-3">
                    <svg
                      :class="[
                        'w-4 h-4 text-gray-400 transition-transform',
                        expandedTypes.has(tt.id) ? 'rotate-90' : '',
                      ]"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                    </svg>
                    <div class="text-left">
                      <h3 class="font-semibold text-gray-900">{{ tt.name }}</h3>
                      <p v-if="tt.description" class="text-sm text-gray-500 line-clamp-1">
                        {{ tt.description }}
                      </p>
                    </div>
                  </div>
                  <span class="text-sm text-gray-400">{{ tt.example_count }} 个案例</span>
                </button>

                <!-- Expanded Examples -->
                <div v-if="expandedTypes.has(tt.id)" class="border-t border-gray-100">
                  <!-- Principle if available -->
                  <div v-if="tt.principle" class="px-4 py-3 bg-gray-50 border-b border-gray-100">
                    <h4 class="text-xs font-medium text-gray-500 mb-1">原理</h4>
                    <p class="text-sm text-gray-700">{{ tt.principle }}</p>
                  </div>

                  <!-- Examples List -->
                  <div class="divide-y divide-gray-50">
                    <div
                      v-for="example in tt.examples"
                      :key="example.id"
                      @click="selectExample(example, tt)"
                      :class="[
                        'px-4 py-3 hover:bg-gray-50 cursor-pointer flex items-start gap-3 transition-colors',
                        selectedExample?.id === example.id ? 'bg-primary-50' : '',
                      ]"
                    >
                      <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2">
                          <span
                            v-if="example.is_featured"
                            class="text-yellow-500 text-sm"
                            title="精选案例"
                          >
                            *
                          </span>
                          <span class="font-medium text-gray-900 truncate">{{ example.title }}</span>
                        </div>
                        <p class="text-sm text-gray-500 line-clamp-1 mt-0.5">
                          {{ example.description }}
                        </p>
                      </div>
                      <span class="text-xs text-gray-400 shrink-0">
                        第{{ example.chapter_index + 1 }}章
                      </span>
                    </div>
                  </div>

                  <div v-if="tt.examples.length === 0" class="px-4 py-6 text-center text-gray-400 text-sm">
                    暂无案例
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Detail Panel -->
      <aside
        v-if="selectedExample && selectedType"
        class="w-96 border-l border-gray-200 bg-white overflow-y-auto shrink-0"
      >
        <div class="p-6">
          <div class="flex items-start justify-between mb-4">
            <div>
              <span :class="['px-2 py-1 text-xs rounded-full', getCategoryColor(selectedType.category).bg, getCategoryColor(selectedType.category).text]">
                {{ selectedType.name }}
              </span>
              <span
                v-if="selectedExample.is_featured"
                class="ml-2 text-yellow-500"
                title="精选案例"
              >
                *
              </span>
            </div>
            <button
              @click="closeDetail"
              class="p-1 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded transition-colors"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
              </svg>
            </button>
          </div>

          <h2 class="text-xl font-bold text-gray-900 mb-4">{{ selectedExample.title }}</h2>

          <!-- Chapter Link -->
          <button
            @click="goToChapter(selectedExample.chapter_id)"
            class="mb-4 text-sm text-primary-600 hover:text-primary-700 flex items-center gap-1"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
            </svg>
            第{{ selectedExample.chapter_index + 1 }}章 {{ selectedExample.chapter_title || '' }}
          </button>

          <div class="space-y-4">
            <div>
              <h4 class="text-sm font-medium text-gray-500 mb-1">描述</h4>
              <p class="text-gray-700">{{ selectedExample.description }}</p>
            </div>

            <div v-if="selectedExample.mechanism">
              <h4 class="text-sm font-medium text-gray-500 mb-1">实现机制</h4>
              <p class="text-gray-700">{{ selectedExample.mechanism }}</p>
            </div>

            <div v-if="selectedExample.evidence.length > 0">
              <h4 class="text-sm font-medium text-gray-500 mb-2">原文证据</h4>
              <div class="space-y-2">
                <blockquote
                  v-for="(evidence, index) in selectedExample.evidence"
                  :key="index"
                  class="pl-3 border-l-2 border-yellow-300 text-sm text-gray-600 italic"
                >
                  "{{ evidence }}"
                </blockquote>
              </div>
            </div>
          </div>

          <div class="mt-6 pt-4 border-t border-gray-100 space-y-2">
            <button
              @click="toggleFeatured(selectedExample.id, !selectedExample.is_featured)"
              :class="[
                'w-full px-4 py-2 rounded-lg transition-colors text-sm font-medium flex items-center justify-center gap-2',
                selectedExample.is_featured
                  ? 'text-yellow-700 bg-yellow-50 hover:bg-yellow-100'
                  : 'text-gray-600 hover:bg-gray-100',
              ]"
            >
              <span>{{ selectedExample.is_featured ? '*' : '+' }}</span>
              {{ selectedExample.is_featured ? '取消精选' : '标记为精选' }}
            </button>
            <button
              @click="deleteExample(selectedExample.id)"
              class="w-full px-4 py-2 text-red-600 hover:bg-red-50 rounded-lg transition-colors text-sm font-medium"
            >
              删除案例
            </button>
          </div>
        </div>
      </aside>
    </div>

    <!-- Clear Library Confirmation Dialog -->
    <div
      v-if="showClearConfirm"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      @click.self="cancelClear"
    >
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl p-6 max-w-md mx-4">
        <h3 class="text-lg font-bold text-gray-900 dark:text-white mb-2">清空技法库</h3>
        <p class="text-gray-600 dark:text-gray-400 mb-6">
          确定要清空全部 {{ stats.example_count }} 个技法案例吗？此操作无法撤销。
        </p>
        <div class="flex justify-end gap-3">
          <button
            @click="cancelClear"
            class="px-4 py-2 text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            :disabled="isClearing"
          >
            取消
          </button>
          <button
            @click="executeClearLibrary"
            class="px-4 py-2 bg-red-600 text-white hover:bg-red-700 rounded-lg transition-colors flex items-center gap-2"
            :disabled="isClearing"
          >
            <span v-if="isClearing" class="animate-spin">...</span>
            清空
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.line-clamp-1 {
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
