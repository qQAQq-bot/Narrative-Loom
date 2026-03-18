<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useTauri } from '@/composables/useTauri';
import BlueprintNode from './BlueprintNode.vue';
import type { BlueprintEvent } from './BlueprintNode.vue';
import { IMPORTANCE_CLASSES, IMPORTANCE_LABELS } from '@/constants/labels';

interface ChapterBlueprint {
  id: string;
  index: number;
  title: string | null;
  analyzed: boolean;
  events: BlueprintEvent[];
}

interface BlueprintData {
  bookId: string;
  totalChapters: number;
  analyzedChapters: number;
  chapters: ChapterBlueprint[];
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
const blueprintData = ref<BlueprintData | null>(null);
const selectedEvent = ref<BlueprintEvent | null>(null);
const hoveredEvent = ref<BlueprintEvent | null>(null);
const expandedChapters = ref<Set<string>>(new Set());
const { invoke } = useTauri();

// Transform snake_case from backend to camelCase
function transformEvent(e: any): BlueprintEvent {
  return {
    id: e.id,
    title: e.title,
    description: e.description,
    importance: e.importance,
    isTurningPoint: e.is_turning_point,
    charactersInvolved: e.characters_involved || [],
    timeMarker: e.time_marker,
  };
}

function transformChapter(c: any): ChapterBlueprint {
  return {
    id: c.id,
    index: c.index,
    title: c.title,
    analyzed: c.analyzed,
    events: (c.events || []).map(transformEvent),
  };
}

async function fetchBlueprint() {
  if (!props.bookId) return;

  loading.value = true;
  error.value = null;

  try {
    const data = await invoke<any>('get_story_blueprint', { bookId: props.bookId });
    blueprintData.value = {
      bookId: data.book_id,
      totalChapters: data.total_chapters,
      analyzedChapters: data.analyzed_chapters,
      chapters: (data.chapters || []).map(transformChapter),
    };
    // Auto-expand chapters with events
    blueprintData.value.chapters.forEach(ch => {
      if (ch.events.length > 0) {
        expandedChapters.value.add(ch.id);
      }
    });
  } catch (e) {
    console.error('Failed to fetch blueprint:', e);
    error.value = typeof e === 'string' ? e : '加载蓝图数据失败';
  } finally {
    loading.value = false;
  }
}

onMounted(fetchBlueprint);
watch(() => props.bookId, fetchBlueprint);

// Expose refresh method for parent components
defineExpose({
  refresh: fetchBlueprint,
});

function handleEventClick(event: BlueprintEvent) {
  selectedEvent.value = event;
}

function handleEventHover(event: BlueprintEvent | null) {
  hoveredEvent.value = event;
}

function closeEventModal() {
  selectedEvent.value = null;
}

// Handle wheel event for horizontal scrolling
function handleWheel(e: WheelEvent) {
  const container = e.currentTarget as HTMLElement;
  if (e.deltaY !== 0) {
    e.preventDefault();
    container.scrollLeft += e.deltaY;
  }
}

function toggleChapter(chapterId: string) {
  if (expandedChapters.value.has(chapterId)) {
    expandedChapters.value.delete(chapterId);
  } else {
    expandedChapters.value.add(chapterId);
  }
}

// Get chapters with events for the list
const chaptersWithEvents = computed(() => {
  if (!blueprintData.value) return [];
  return blueprintData.value.chapters.filter(ch => ch.analyzed && ch.events.length > 0);
});

const progressPercent = computed(() => {
  if (!blueprintData.value || blueprintData.value.totalChapters === 0) return 0;
  return Math.round((blueprintData.value.analyzedChapters / blueprintData.value.totalChapters) * 100);
});

const getImportanceLabel = (importance: string) => {
  return IMPORTANCE_LABELS[importance] || importance;
};

const getImportanceClass = (importance: string) => {
  // Keep a stable default even if backend sends unexpected value
  return IMPORTANCE_CLASSES[importance] || IMPORTANCE_CLASSES.normal;
};
</script>

<template>
  <div class="blueprint-timeline h-full flex flex-col">
    <!-- Header -->
    <div class="flex-shrink-0 px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-white/50 dark:bg-gray-800/50 backdrop-blur-sm">
      <div class="flex items-center justify-between">
        <!-- View Mode Switcher -->
        <div class="inline-flex items-center gap-1 p-1 bg-fabric-sand/20 rounded-full">
          <button
            :class="[
              'flex items-center gap-2 px-3.5 py-1.5 text-sm font-medium rounded-full transition-all duration-220',
              'bg-accent-timeline/15 text-accent-timeline shadow-sm'
            ]"
          >
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 17h8m0 0V9m0 8l-8-8-4 4-6-6" />
            </svg>
            时间线
          </button>
          <button
            @click="emit('update:viewMode', 'graph')"
            class="flex items-center gap-2 px-3.5 py-1.5 text-sm font-medium rounded-full transition-all duration-220 text-fabric-thread hover:text-fabric-sepia"
          >
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
            人物关系
          </button>
        </div>
        <div class="flex items-center gap-4">
          <!-- Progress indicator -->
          <div class="flex items-center gap-2 text-sm">
            <span class="text-gray-500 dark:text-gray-400">分析进度</span>
            <div class="w-24 h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
              <div
                class="h-full bg-gradient-to-r from-blue-400 to-blue-600 transition-all duration-500"
                :style="{ width: `${progressPercent}%` }"
              ></div>
            </div>
            <span class="text-gray-600 dark:text-gray-300 font-medium">
              {{ blueprintData?.analyzedChapters || 0 }}/{{ blueprintData?.totalChapters || 0 }}
            </span>
          </div>
        </div>
      </div>

      <!-- Legend -->
      <div class="flex items-center gap-4 mt-3 text-xs text-gray-500 dark:text-gray-400">
        <div class="flex items-center gap-1.5">
          <div class="w-3 h-3 rounded-full bg-red-400 border-2 border-red-500"></div>
          <span>关键事件</span>
        </div>
        <div class="flex items-center gap-1.5">
          <div class="w-2.5 h-2.5 rounded-full bg-blue-400 border-2 border-blue-500"></div>
          <span>重要事件</span>
        </div>
        <div class="flex items-center gap-1.5">
          <div class="w-2 h-2 rounded-full bg-slate-400 border-2 border-slate-500"></div>
          <span>普通事件</span>
        </div>
        <div class="flex items-center gap-1.5">
          <div class="w-3 h-3 rounded-full bg-amber-400 border-2 border-amber-500 relative">
            <svg class="absolute -top-0.5 -right-0.5 w-2 h-2 text-amber-500" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
            </svg>
          </div>
          <span>转折点</span>
        </div>
        <div class="flex items-center gap-1.5">
          <div class="w-8 border-t-2 border-dashed border-gray-300 dark:border-gray-600"></div>
          <span>未分析</span>
        </div>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <div class="flex flex-col items-center gap-3 text-gray-400">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-400"></div>
        <span class="text-sm">加载蓝图数据中...</span>
      </div>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="flex-1 flex items-center justify-center">
      <div class="text-center text-gray-500 dark:text-gray-400">
        <svg class="w-12 h-12 mx-auto mb-3 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <p>{{ error }}</p>
        <button @click="fetchBlueprint" class="mt-3 text-blue-500 hover:text-blue-600 text-sm">
          重新加载
        </button>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else-if="!blueprintData || blueprintData.chapters.length === 0" class="flex-1 flex items-center justify-center">
      <div class="text-center text-gray-400 dark:text-gray-500">
        <svg class="w-16 h-16 mx-auto mb-4 text-gray-300 dark:text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l5.447 2.724A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7" />
        </svg>
        <p class="text-lg font-medium">暂无蓝图数据</p>
        <p class="text-sm mt-2">开始分析章节后，故事脉络将在这里展示</p>
      </div>
    </div>

    <!-- Timeline content -->
    <div v-else class="flex-1 flex flex-col overflow-hidden">
      <!-- Blueprint track (horizontal scrollable) -->
      <div
        class="shrink-0 overflow-x-auto overflow-y-hidden px-6 py-4 border-b border-gray-200 dark:border-gray-700"
        @wheel="handleWheel"
      >
        <div class="min-w-max flex items-center">
          <div class="flex items-center gap-0">
            <template v-for="(chapter, idx) in blueprintData.chapters" :key="chapter.id">
              <!-- Chapter section -->
              <div class="chapter-section flex flex-col items-center min-w-[140px] px-2">
                <!-- Chapter label -->
                <div
                  class="text-xs font-medium mb-3 text-center max-w-[120px] truncate"
                  :class="chapter.analyzed ? 'text-gray-700 dark:text-gray-300' : 'text-gray-400 dark:text-gray-500'"
                  :title="chapter.title || `第${chapter.index}章`"
                >
                  {{ chapter.title || `第${chapter.index}章` }}
                </div>

                <!-- Events track -->
                <div class="relative flex items-center h-12">
                  <!-- Connecting line -->
                  <div
                    class="absolute top-1/2 left-0 right-0 h-0.5 -translate-y-1/2"
                    :class="chapter.analyzed ? 'bg-gray-300 dark:bg-gray-600' : 'border-t-2 border-dashed border-gray-200 dark:border-gray-700'"
                  ></div>

                  <!-- Events -->
                  <div v-if="chapter.analyzed && chapter.events.length > 0" class="relative flex items-center gap-2 px-2">
                    <BlueprintNode
                      v-for="event in chapter.events"
                      :key="event.id"
                      :event="event"
                      @click="handleEventClick"
                      @hover="handleEventHover"
                    />
                  </div>

                  <!-- Placeholder for analyzed but no events -->
                  <div v-else-if="chapter.analyzed" class="relative w-full flex justify-center">
                    <div class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"></div>
                  </div>

                  <!-- Unanalyzed placeholder -->
                  <div v-else class="relative w-full flex justify-center">
                    <div class="text-[10px] text-gray-400 dark:text-gray-500 bg-white dark:bg-gray-800 px-2">
                      未分析
                    </div>
                  </div>
                </div>

                <!-- Event count -->
                <div class="text-[10px] mt-2" :class="chapter.analyzed ? 'text-gray-500 dark:text-gray-400' : 'text-transparent'">
                  {{ chapter.events.length }} 个事件
                </div>
              </div>

              <!-- Arrow connector between chapters -->
              <div v-if="idx < blueprintData.chapters.length - 1" class="flex items-center h-12 -mx-1">
                <svg class="w-4 h-4 text-gray-300 dark:text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
              </div>
            </template>
          </div>
        </div>
      </div>

      <!-- Event list (below timeline) -->
      <div class="flex-1 overflow-y-auto p-6">
        <div v-if="chaptersWithEvents.length === 0" class="text-center text-gray-400 dark:text-gray-500 py-8">
          <p class="text-sm">暂无事件数据</p>
          <p class="text-xs mt-1">分析章节后，事件将在这里显示</p>
        </div>
        <div v-else class="space-y-3">
          <div v-for="chapter in chaptersWithEvents" :key="chapter.id" class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
            <!-- Chapter header (collapsible) -->
            <button
              @click="toggleChapter(chapter.id)"
              class="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
            >
              <div class="flex items-center gap-3">
                <svg
                  class="w-4 h-4 text-gray-400 transition-transform"
                  :class="{ 'rotate-90': expandedChapters.has(chapter.id) }"
                  fill="none" viewBox="0 0 24 24" stroke="currentColor"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
                <span class="font-medium text-gray-700 dark:text-gray-300">
                  {{ chapter.title || `第${chapter.index}章` }}
                </span>
                <span class="text-xs text-gray-400 dark:text-gray-500">
                  {{ chapter.events.length }} 个事件
                </span>
              </div>
            </button>

            <!-- Events list -->
            <div v-if="expandedChapters.has(chapter.id)" class="border-t border-gray-100 dark:border-gray-700">
              <div
                v-for="event in chapter.events"
                :key="event.id"
                @click="handleEventClick(event)"
                class="px-4 py-3 flex items-start gap-3 hover:bg-gray-50 dark:hover:bg-gray-700/30 cursor-pointer border-b border-gray-50 dark:border-gray-700/50 last:border-b-0"
              >
                <!-- Importance indicator -->
                <div class="mt-1 shrink-0">
                  <div
                    class="w-2.5 h-2.5 rounded-full"
                    :class="{
                      'bg-red-400': event.importance === 'critical',
                      'bg-blue-400': event.importance === 'major',
                      'bg-gray-400': event.importance === 'normal' || event.importance === 'minor',
                    }"
                  ></div>
                </div>
                <!-- Event content -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-medium text-gray-800 dark:text-gray-200 text-sm">{{ event.title }}</span>
                    <span
                      v-if="event.isTurningPoint"
                      class="px-1.5 py-0.5 text-[10px] font-bold rounded bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
                    >
                      转折点
                    </span>
                    <span :class="['px-1.5 py-0.5 text-[10px] rounded', getImportanceClass(event.importance)]">
                      {{ getImportanceLabel(event.importance) }}
                    </span>
                  </div>
                  <p v-if="event.description" class="text-xs text-gray-500 dark:text-gray-400 mt-1 line-clamp-2">
                    {{ event.description }}
                  </p>
                  <div v-if="event.timeMarker" class="text-[10px] text-gray-400 dark:text-gray-500 mt-1">
                    ⏱ {{ event.timeMarker }}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Event detail modal -->
    <Teleport to="body">
      <div
        v-if="selectedEvent"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
        @click.self="closeEventModal"
      >
        <div class="bg-white dark:bg-gray-800 rounded-2xl shadow-xl max-w-lg w-full mx-4 overflow-hidden">
          <!-- Modal header -->
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-start justify-between">
            <div>
              <div class="flex items-center gap-2">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                  {{ selectedEvent.title }}
                </h3>
                <span
                  v-if="selectedEvent.isTurningPoint"
                  class="px-2 py-0.5 text-[10px] font-bold rounded-full bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
                >
                  转折点
                </span>
              </div>
              <div class="flex items-center gap-2 mt-1">
                <span
                  class="px-2 py-0.5 text-[10px] font-medium rounded-full"
                  :class="{
                    'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400': selectedEvent.importance === 'critical',
                    'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400': selectedEvent.importance === 'major',
                    'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-400': selectedEvent.importance === 'normal' || selectedEvent.importance === 'minor',
                  }"
                >
                  {{ getImportanceLabel(selectedEvent.importance) }}
                </span>
                <span v-if="selectedEvent.timeMarker" class="text-xs text-gray-500 dark:text-gray-400">
                  {{ selectedEvent.timeMarker }}
                </span>
              </div>
            </div>
            <button
              @click="closeEventModal"
              class="p-1 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            >
              <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Modal body -->
          <div class="px-6 py-4 max-h-[60vh] overflow-y-auto">
            <!-- Description -->
            <div v-if="selectedEvent.description" class="mb-4">
              <h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">描述</h4>
              <p class="text-sm text-gray-700 dark:text-gray-300 leading-relaxed">
                {{ selectedEvent.description }}
              </p>
            </div>

            <!-- Characters involved -->
            <div v-if="selectedEvent.charactersInvolved.length > 0">
              <h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">相关人物</h4>
              <div class="flex flex-wrap gap-2">
                <span
                  v-for="char in selectedEvent.charactersInvolved"
                  :key="char"
                  class="px-2.5 py-1 text-xs font-medium rounded-full bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400"
                >
                  {{ char }}
                </span>
              </div>
            </div>
          </div>

          <!-- Modal footer -->
          <div class="px-6 py-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
            <button
              @click="closeEventModal"
              class="w-full py-2 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors"
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
.blueprint-timeline {
  background: linear-gradient(to bottom, rgb(249 250 251 / 0.5), rgb(243 244 246 / 0.3));
}

.dark .blueprint-timeline {
  background: linear-gradient(to bottom, rgb(17 24 39 / 0.5), rgb(31 41 55 / 0.3));
}

.chapter-section:hover {
  background: rgba(59, 130, 246, 0.05);
  border-radius: 0.75rem;
}

/* Custom scrollbar for horizontal scroll */
.overflow-x-auto::-webkit-scrollbar {
  height: 8px;
}

.overflow-x-auto::-webkit-scrollbar-track {
  background: transparent;
}

.overflow-x-auto::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.3);
  border-radius: 4px;
}

.overflow-x-auto::-webkit-scrollbar-thumb:hover {
  background-color: rgba(156, 163, 175, 0.5);
}
</style>
