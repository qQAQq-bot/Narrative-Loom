<script setup lang="ts">
import { computed } from 'vue';
import type { KnowledgeCardInfo } from '@/stores/analysis';

const props = defineProps<{
  card: KnowledgeCardInfo;
  selected?: boolean;
}>();

const emit = defineEmits<{
  (e: 'select'): void;
  (e: 'updateStatus', status: string): void;
  (e: 'delete'): void;
  (e: 'evidence-click', index: number, excerpt: string): void;
}>();

const knowledgeTypeLabel = computed(() => {
  const typeMap: Record<string, string> = {
    character: '人物',
    setting: '设定',
    event: '事件',
    relationship: '关系',
    worldbuilding: '世界观',
    other: '其他',
  };
  return typeMap[props.card.knowledge_type] || props.card.knowledge_type;
});

const themeColors = computed(() => {
  const colorMap: Record<string, { bg: string; text: string; border: string; icon: string }> = {
    character: { bg: 'bg-blue-50', text: 'text-blue-700', border: 'border-blue-200', icon: '👤' },
    setting: { bg: 'bg-emerald-50', text: 'text-emerald-700', border: 'border-emerald-200', icon: '🏛️' },
    event: { bg: 'bg-orange-50', text: 'text-orange-700', border: 'border-orange-200', icon: '📅' },
    relationship: { bg: 'bg-pink-50', text: 'text-pink-700', border: 'border-pink-200', icon: '🔗' },
    worldbuilding: { bg: 'bg-teal-50', text: 'text-teal-700', border: 'border-teal-200', icon: '🌍' },
    other: { bg: 'bg-gray-50', text: 'text-gray-700', border: 'border-gray-200', icon: '📄' },
  };
  return colorMap[props.card.knowledge_type] || colorMap.other;
});

const statusConfig = computed(() => {
  const statusMap: Record<string, { label: string; bg: string; text: string; icon: string }> = {
    pending: { label: '待审核', bg: 'bg-amber-50', text: 'text-amber-700', icon: '⏳' },
    accepted: { label: '已确认', bg: 'bg-green-50', text: 'text-green-700', icon: '✓' },
    rejected: { label: '已拒绝', bg: 'bg-red-50', text: 'text-red-700', icon: '✗' },
    merged: { label: '已合并', bg: 'bg-blue-50', text: 'text-blue-700', icon: '⊕' },
  };
  return statusMap[props.card.status] || { label: props.card.status, bg: 'bg-gray-50', text: 'text-gray-700', icon: '?' };
});

const confidenceConfig = computed(() => {
  const confMap: Record<string, { label: string; color: string; width: string }> = {
    high: { label: '高', color: 'bg-green-500', width: 'w-full' },
    medium: { label: '中', color: 'bg-amber-500', width: 'w-2/3' },
    low: { label: '低', color: 'bg-red-500', width: 'w-1/3' },
  };
  return confMap[props.card.confidence] || { label: props.card.confidence, color: 'bg-gray-400', width: 'w-1/2' };
});

// Extract display content from the content object
const displayContent = computed(() => {
  const content = props.card.content;
  if (typeof content === 'string') return content;

  // For character type, show name and description
  if (props.card.knowledge_type === 'character') {
    const desc = (content as Record<string, unknown>).description || '';
    return desc;
  }

  // For setting type
  if (props.card.knowledge_type === 'setting') {
    const desc = (content as Record<string, unknown>).description || '';
    return desc;
  }

  // For event type
  if (props.card.knowledge_type === 'event') {
    return (content as Record<string, unknown>).description || '';
  }

  // Default: stringify the content
  return JSON.stringify(content).slice(0, 100);
});

function handleEvidenceClick(e: Event, index: number) {
  e.stopPropagation();
  emit('evidence-click', index, props.card.evidence[index]);
}

function handleStatusUpdate(e: Event, status: string) {
  e.stopPropagation();
  emit('updateStatus', status);
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
          <div class="flex items-center gap-2 mb-1.5 flex-wrap">
            <!-- Type Badge -->
            <span
              class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium border"
              :class="[themeColors.bg, themeColors.text, themeColors.border]"
            >
              <span>{{ themeColors.icon }}</span>
              {{ knowledgeTypeLabel }}
            </span>
            <!-- Status Badge -->
            <span
              class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
              :class="[statusConfig.bg, statusConfig.text]"
            >
              <span class="text-[10px]">{{ statusConfig.icon }}</span>
              {{ statusConfig.label }}
            </span>
          </div>
          <h3 class="text-base font-bold text-gray-900 leading-tight truncate group-hover:text-primary-600 transition-colors">
            {{ card.title }}
          </h3>
        </div>

        <!-- Delete Button -->
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

      <!-- Description Section -->
      <div class="mb-4 flex-1">
        <p class="text-sm text-gray-700 leading-relaxed line-clamp-3">
          {{ displayContent }}
        </p>
      </div>

      <!-- Evidence Preview -->
      <div v-if="card.evidence.length > 0" class="mb-3 pt-3 border-t border-gray-100">
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

      <!-- Footer Section -->
      <div class="flex items-center justify-between pt-3 border-t border-gray-100">
        <!-- Confidence Indicator -->
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-500">可信度</span>
          <div class="w-16 h-1.5 bg-gray-200 rounded-full overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-300"
              :class="[confidenceConfig.color, confidenceConfig.width]"
            ></div>
          </div>
          <span class="text-xs text-gray-600 font-medium">{{ confidenceConfig.label }}</span>
        </div>

        <!-- Status Actions (only for pending) -->
        <div
          v-if="card.status === 'pending'"
          class="flex items-center gap-1"
        >
          <button
            @click="handleStatusUpdate($event, 'accepted')"
            class="p-1.5 rounded-full text-gray-400 hover:text-green-600 hover:bg-green-50 transition-all"
            title="确认"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
              <path fill-rule="evenodd" d="M16.704 4.153a.75.75 0 01.143 1.052l-8 10.5a.75.75 0 01-1.127.075l-4.5-4.5a.75.75 0 011.06-1.06l3.894 3.893 7.48-9.817a.75.75 0 011.05-.143z" clip-rule="evenodd" />
            </svg>
          </button>
          <button
            @click="handleStatusUpdate($event, 'rejected')"
            class="p-1.5 rounded-full text-gray-400 hover:text-red-600 hover:bg-red-50 transition-all"
            title="拒绝"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
              <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
            </svg>
          </button>
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
