<script setup lang="ts">
import { computed } from 'vue';

export interface HighlightRange {
  start: number;
  end: number;
  evidenceId?: string;
  color?: string;
  borderColor?: string;
}

const props = defineProps<{
  text: string;
  highlights?: HighlightRange[];
  activeHighlightId?: string | null;
}>();

const emit = defineEmits<{
  (e: 'highlight-click', evidenceId: string): void;
}>();

interface TextSegment {
  text: string;
  isHighlight: boolean;
  evidenceId?: string;
  isActive: boolean;
  color?: string;
  borderColor?: string;
  isEntity: boolean;
}

const segments = computed<TextSegment[]>(() => {
  if (!props.highlights || props.highlights.length === 0) {
    return [{ text: props.text, isHighlight: false, isActive: false, isEntity: false }];
  }

  // Sort highlights by start position
  const sortedHighlights = [...props.highlights].sort((a, b) => a.start - b.start);

  const result: TextSegment[] = [];
  let currentPos = 0;

  for (const highlight of sortedHighlights) {
    // Validate highlight range
    if (highlight.start < 0 || highlight.end > props.text.length || highlight.start >= highlight.end) {
      continue;
    }

    // Add non-highlighted text before this highlight
    if (highlight.start > currentPos) {
      result.push({
        text: props.text.slice(currentPos, highlight.start),
        isHighlight: false,
        isActive: false,
        isEntity: false,
      });
    }

    // Check if this is an entity highlight
    const isEntity = highlight.evidenceId?.startsWith('entity-') ?? false;

    // Add highlighted text
    result.push({
      text: props.text.slice(highlight.start, highlight.end),
      isHighlight: true,
      evidenceId: highlight.evidenceId,
      isActive: highlight.evidenceId === props.activeHighlightId,
      color: highlight.color,
      borderColor: highlight.borderColor,
      isEntity,
    });

    currentPos = highlight.end;
  }

  // Add remaining text after last highlight
  if (currentPos < props.text.length) {
    result.push({
      text: props.text.slice(currentPos),
      isHighlight: false,
      isActive: false,
      isEntity: false,
    });
  }

  return result;
});

function handleHighlightClick(evidenceId?: string) {
  if (evidenceId) {
    emit('highlight-click', evidenceId);
  }
}
</script>

<template>
  <span class="text-highlight">
    <template v-for="(segment, index) in segments" :key="index">
      <span
        v-if="segment.isHighlight"
        :class="[
          'highlight-segment cursor-pointer transition-all duration-200',
          segment.isEntity
            ? 'entity-highlight hover:opacity-80'
            : segment.isActive
              ? 'bg-yellow-300 ring-2 ring-yellow-500'
              : 'bg-yellow-100 hover:bg-yellow-200',
        ]"
        :style="segment.color ? {
          backgroundColor: segment.color,
          borderBottom: segment.isEntity ? '2px solid currentColor' : undefined,
        } : undefined"
        @click="handleHighlightClick(segment.evidenceId)"
      >{{ segment.text }}</span>
      <template v-else>{{ segment.text }}</template>
    </template>
  </span>
</template>

<style scoped>
.highlight-segment {
  border-radius: 2px;
  padding: 0 1px;
  margin: 0 -1px;
}

.entity-highlight {
  border-radius: 3px;
  padding: 1px 2px;
  margin: 0 -2px;
}
</style>
