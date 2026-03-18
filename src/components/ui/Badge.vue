<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  variant: 'importing' | 'ready' | 'analyzing' | 'completed' | 'error' | string;
}>();

const config = computed(() => {
  // Handle complex error strings (e.g. "error: invalid file")
  const status = props.variant.startsWith('error') ? 'error' : props.variant;

  switch (status) {
    case 'importing':
      return { class: 'bg-yellow-50 text-yellow-700 border-yellow-200', label: '导入中', icon: '📥' };
    case 'ready':
      return { class: 'bg-blue-50 text-blue-700 border-blue-200', label: '待分析', icon: '⏱️' };
    case 'analyzing':
      return { class: 'bg-purple-50 text-purple-700 border-purple-200', label: '分析中', icon: '⚡' };
    case 'completed':
      return { class: 'bg-green-50 text-green-700 border-green-200', label: '已完成', icon: '✓' };
    case 'error':
      return { class: 'bg-red-50 text-red-700 border-red-200', label: '错误', icon: '!' };
    default:
      return { class: 'bg-gray-50 text-gray-700 border-gray-200', label: props.variant, icon: '' };
  }
});
</script>

<template>
  <span 
    class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border"
    :class="config.class"
  >
    <span class="mr-1.5 opacity-70" v-if="config.icon">{{ config.icon }}</span>
    {{ config.label }}
  </span>
</template>
