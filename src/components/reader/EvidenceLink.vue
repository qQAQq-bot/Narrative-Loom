<script setup lang="ts">
import { computed } from 'vue';

export interface EvidenceInfo {
  id: string;
  excerpt: string;
  chapterId?: string;
  paragraphIndex?: number;
}

const props = defineProps<{
  evidence: EvidenceInfo;
  active?: boolean;
  compact?: boolean;
}>();

const emit = defineEmits<{
  (e: 'click', evidence: EvidenceInfo): void;
  (e: 'hover-start', evidence: EvidenceInfo): void;
  (e: 'hover-end', evidence: EvidenceInfo): void;
}>();

const truncatedExcerpt = computed(() => {
  const maxLength = props.compact ? 30 : 60;
  if (props.evidence.excerpt.length <= maxLength) {
    return props.evidence.excerpt;
  }
  return props.evidence.excerpt.slice(0, maxLength) + '...';
});

function handleClick() {
  emit('click', props.evidence);
}

function handleMouseEnter() {
  emit('hover-start', props.evidence);
}

function handleMouseLeave() {
  emit('hover-end', props.evidence);
}
</script>

<template>
  <button
    type="button"
    :class="[
      'evidence-link group text-left transition-all duration-200',
      'px-2 py-1.5 rounded-md border',
      active
        ? 'bg-yellow-50 border-yellow-300 text-yellow-800'
        : 'bg-gray-50 border-gray-200 text-gray-600 hover:bg-yellow-50 hover:border-yellow-200 hover:text-yellow-700',
      compact ? 'text-xs' : 'text-sm',
    ]"
    @click="handleClick"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
  >
    <span class="flex items-start gap-1.5">
      <span
        :class="[
          'shrink-0 mt-0.5',
          active ? 'text-yellow-500' : 'text-gray-400 group-hover:text-yellow-500',
        ]"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 20 20"
          fill="currentColor"
          class="w-3.5 h-3.5"
        >
          <path
            fill-rule="evenodd"
            d="M15.621 4.379a3 3 0 00-4.242 0l-7 7a3 3 0 004.241 4.243h.001l.497-.5a.75.75 0 011.064 1.057l-.498.501-.002.002a4.5 4.5 0 01-6.364-6.364l7-7a4.5 4.5 0 016.368 6.36l-3.455 3.553A2.625 2.625 0 119.52 9.52l3.45-3.451a.75.75 0 111.061 1.06l-3.45 3.451a1.125 1.125 0 001.587 1.595l3.454-3.553a3 3 0 000-4.242z"
            clip-rule="evenodd"
          />
        </svg>
      </span>
      <span class="evidence-excerpt italic">
        "{{ truncatedExcerpt }}"
      </span>
    </span>
  </button>
</template>

<style scoped>
.evidence-link {
  display: inline-flex;
  max-width: 100%;
}

.evidence-excerpt {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-all;
}
</style>
