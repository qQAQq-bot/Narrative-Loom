<script setup lang="ts">
import { computed } from 'vue';

export interface BlueprintEvent {
  id: string;
  title: string;
  description?: string;
  importance: string;
  isTurningPoint: boolean;
  charactersInvolved: string[];
  timeMarker?: string;
}

const props = defineProps<{
  event: BlueprintEvent;
  showDetails?: boolean;
}>();

const emit = defineEmits<{
  (e: 'click', event: BlueprintEvent): void;
  (e: 'hover', event: BlueprintEvent | null): void;
}>();

const nodeClass = computed(() => {
  const classes = ['blueprint-node', 'transition-all', 'duration-200', 'cursor-pointer'];

  if (props.event.isTurningPoint) {
    classes.push('turning-point');
  }

  switch (props.event.importance) {
    case 'critical':
      classes.push('importance-critical');
      break;
    case 'major':
      classes.push('importance-major');
      break;
    case 'minor':
      classes.push('importance-minor');
      break;
    default:
      classes.push('importance-normal');
  }

  return classes.join(' ');
});

const nodeSize = computed(() => {
  switch (props.event.importance) {
    case 'critical':
      return 'w-5 h-5';
    case 'major':
      return 'w-4 h-4';
    case 'minor':
      return 'w-2.5 h-2.5';
    default:
      return 'w-3 h-3';
  }
});

const nodeColor = computed(() => {
  if (props.event.isTurningPoint) {
    return 'bg-amber-400 border-amber-500 shadow-amber-200';
  }
  switch (props.event.importance) {
    case 'critical':
      return 'bg-red-400 border-red-500 shadow-red-200';
    case 'major':
      return 'bg-blue-400 border-blue-500 shadow-blue-200';
    case 'minor':
      return 'bg-gray-300 border-gray-400';
    default:
      return 'bg-slate-400 border-slate-500';
  }
});
</script>

<template>
  <div
    :class="nodeClass"
    class="relative group"
    @click="emit('click', event)"
    @mouseenter="emit('hover', event)"
    @mouseleave="emit('hover', null)"
  >
    <!-- Node dot -->
    <div
      class="rounded-full border-2 shadow-sm hover:scale-125 transition-transform"
      :class="[nodeSize, nodeColor]"
    >
      <!-- Turning point star indicator -->
      <svg
        v-if="event.isTurningPoint"
        class="absolute -top-1 -right-1 w-3 h-3 text-amber-500"
        viewBox="0 0 24 24"
        fill="currentColor"
      >
        <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
      </svg>
    </div>

    <!-- Hover tooltip -->
    <div
      class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-2 bg-gray-900 dark:bg-gray-700 text-white text-xs rounded-lg shadow-lg opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap z-50 pointer-events-none max-w-xs"
    >
      <div class="font-medium truncate">{{ event.title }}</div>
      <div v-if="event.timeMarker" class="text-gray-300 text-[10px] mt-0.5">
        {{ event.timeMarker }}
      </div>
      <div v-if="event.charactersInvolved.length > 0" class="text-gray-400 text-[10px] mt-1">
        {{ event.charactersInvolved.slice(0, 3).join(', ') }}
        <span v-if="event.charactersInvolved.length > 3">...</span>
      </div>
      <!-- Arrow -->
      <div class="absolute top-full left-1/2 -translate-x-1/2 border-4 border-transparent border-t-gray-900 dark:border-t-gray-700"></div>
    </div>
  </div>
</template>

<style scoped>
.blueprint-node {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.turning-point > div:first-child {
  animation: pulse-glow 2s ease-in-out infinite;
}

@keyframes pulse-glow {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(251, 191, 36, 0.4);
  }
  50% {
    box-shadow: 0 0 0 4px rgba(251, 191, 36, 0);
  }
}
</style>
