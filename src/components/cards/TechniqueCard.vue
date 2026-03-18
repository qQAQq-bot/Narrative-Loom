<script setup lang="ts">
import { computed } from 'vue';
import type { TechniqueCardInfo } from '@/stores/analysis';

const props = defineProps<{
  card: TechniqueCardInfo;
  selected?: boolean;
}>();

const emit = defineEmits<{
  (e: 'select'): void;
  (e: 'collect'): void;
  (e: 'uncollect'): void;
  (e: 'delete'): void;
  (e: 'evidence-click', index: number, excerpt: string): void;
}>();

const techniqueTypeLabel = computed(() => {
  const typeMap: Record<string, string> = {
    narrative: '叙事技法',
    dialogue: '对话技法',
    description: '描写技法',
    structure: '结构技法',
    pacing: '节奏技法',
    tension: '张力技法',
    foreshadowing: '伏笔技法',
    character: '人物刻画',
    atmosphere: '氛围营造',
    other: '其他技法',
  };
  return typeMap[props.card.technique_type] || props.card.technique_type;
});

const themeColors = computed(() => {
  const colorMap: Record<string, { bg: string; text: string; border: string }> = {
    narrative: { bg: 'bg-blue-50', text: 'text-blue-700', border: 'border-blue-200' },
    dialogue: { bg: 'bg-green-50', text: 'text-green-700', border: 'border-green-200' },
    description: { bg: 'bg-purple-50', text: 'text-purple-700', border: 'border-purple-200' },
    structure: { bg: 'bg-orange-50', text: 'text-orange-700', border: 'border-orange-200' },
    pacing: { bg: 'bg-pink-50', text: 'text-pink-700', border: 'border-pink-200' },
    tension: { bg: 'bg-red-50', text: 'text-red-700', border: 'border-red-200' },
    foreshadowing: { bg: 'bg-amber-50', text: 'text-amber-700', border: 'border-amber-200' },
    character: { bg: 'bg-cyan-50', text: 'text-cyan-700', border: 'border-cyan-200' },
    atmosphere: { bg: 'bg-indigo-50', text: 'text-indigo-700', border: 'border-indigo-200' },
    other: { bg: 'bg-gray-50', text: 'text-gray-700', border: 'border-gray-200' },
  };
  return colorMap[props.card.technique_type] || colorMap.other;
});

function handleCollectClick(e: Event) {
  e.stopPropagation();
  if (props.card.collected) {
    emit('uncollect');
  } else {
    emit('collect');
  }
}

function handleEvidenceClick(e: Event, index: number) {
  e.stopPropagation();
  emit('evidence-click', index, props.card.evidence[index]);
}

function handleDeleteClick(e: Event) {
  e.stopPropagation();
  emit('delete');
}
</script>

<template>
  <div
    @click="emit('select')"
    :class="[
      'relative group rounded-xl border transition-all duration-300 overflow-hidden cursor-pointer bg-white',
      selected
        ? 'border-primary-500 shadow-md ring-1 ring-primary-500'
        : 'border-gray-200 hover:border-gray-300 hover:shadow-lg hover:-translate-y-0.5',
    ]"
  >
    <!-- Card Content Wrapper -->
    <div class="p-4 flex flex-col h-full">
      <!-- Header Section -->
      <div class="flex items-start justify-between gap-3 mb-3">
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2 mb-1.5">
            <span
              class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border"
              :class="[themeColors.bg, themeColors.text, themeColors.border]"
            >
              {{ techniqueTypeLabel }}
            </span>
            <span v-if="card.collected" class="text-xs text-yellow-500 font-medium flex items-center gap-0.5">
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-3 h-3">
                <path fill-rule="evenodd" d="M10.788 3.21c.448-1.077 1.976-1.077 2.424 0l2.082 5.006 5.404.434c1.164.093 1.636 1.545.749 2.305l-4.117 3.527 1.257 5.273c.271 1.136-.964 2.033-1.96 1.425L12 18.354 7.373 21.18c-.996.608-2.231-.29-1.96-1.425l1.257-5.273-4.117-3.527c-.887-.76-.415-2.212.749-2.305l5.404-.434 2.082-5.005Z" clip-rule="evenodd" />
              </svg>
              已收藏
            </span>
          </div>
          <h3 class="text-base font-bold text-gray-900 leading-tight truncate group-hover:text-primary-600 transition-colors">
            {{ card.title }}
          </h3>
        </div>

        <div class="flex items-center gap-1">
          <button
            @click="handleCollectClick"
            class="shrink-0 p-1.5 rounded-full transition-all duration-200 focus:outline-none"
            :class="[
              card.collected
                ? 'text-yellow-500 bg-yellow-50 hover:bg-yellow-100'
                : 'text-gray-300 hover:text-yellow-500 hover:bg-gray-100'
            ]"
            :title="card.collected ? '取消收藏' : '收藏到技法库'"
          >
            <svg v-if="card.collected" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5">
              <path fill-rule="evenodd" d="M10.788 3.21c.448-1.077 1.976-1.077 2.424 0l2.082 5.006 5.404.434c1.164.093 1.636 1.545.749 2.305l-4.117 3.527 1.257 5.273c.271 1.136-.964 2.033-1.96 1.425L12 18.354 7.373 21.18c-.996.608-2.231-.29-1.96-1.425l1.257-5.273-4.117-3.527c-.887-.76-.415-2.212.749-2.305l5.404-.434 2.082-5.005Z" clip-rule="evenodd" />
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 0 1 1.04 0l2.125 5.111a.563.563 0 0 0 .475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 0 0-.182.557l1.285 5.385a.562.562 0 0 1-.84.61l-4.725-2.885a.562.562 0 0 0-.586 0L6.982 20.54a.562.562 0 0 1-.84-.61l1.285-5.386a.562.562 0 0 0-.182-.557l-4.204-3.602a.562.562 0 0 1 .321-.988l5.518-.442a.563.563 0 0 0 .475-.345L11.48 3.5Z" />
            </svg>
          </button>

          <button
            @click="handleDeleteClick"
            class="shrink-0 p-1.5 rounded-full text-gray-300 hover:text-red-500 hover:bg-red-50 transition-all duration-200 focus:outline-none opacity-0 group-hover:opacity-100"
            title="删除卡片"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
              <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Description Section -->
      <div class="mb-4">
        <p class="text-sm text-gray-700 leading-relaxed line-clamp-3">
          {{ card.description }}
        </p>
      </div>

      <!-- Footer Section (Tags & Evidence) -->
      <div class="mt-auto space-y-3">
        <!-- Tags -->
        <div v-if="card.tags.length > 0" class="flex flex-wrap gap-1.5">
          <span
            v-for="tag in card.tags.slice(0, 3)"
            :key="tag"
            class="text-xs px-2 py-0.5 bg-gray-50 text-gray-600 rounded border border-gray-100"
          >
            #{{ tag }}
          </span>
          <span v-if="card.tags.length > 3" class="text-xs text-gray-400 py-0.5">
            +{{ card.tags.length - 3 }}
          </span>
        </div>

        <!-- Evidence Preview (Condensed) -->
        <div v-if="card.evidence.length > 0" class="pt-3 border-t border-gray-100">
          <div
            v-for="(evidence, index) in card.evidence.slice(0, 1)"
            :key="index"
            @click="handleEvidenceClick($event, index)"
            class="group/evidence flex gap-2 items-start p-2 rounded-lg bg-gray-50 hover:bg-primary-50 transition-colors cursor-pointer border border-transparent hover:border-primary-100"
          >
            <span class="text-xs shrink-0 mt-0.5 select-none" role="img" aria-label="evidence">💡</span>
            <span class="text-xs text-gray-600 group-hover/evidence:text-primary-700 italic line-clamp-2">
              "{{ evidence }}"
            </span>
          </div>
          <div v-if="card.evidence.length > 1" class="px-2 mt-1">
             <span class="text-[10px] text-gray-400">还有 {{ card.evidence.length - 1 }} 处引用...</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.line-clamp-3 {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
