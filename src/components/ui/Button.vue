<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(defineProps<{
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  disabled?: boolean;
  type?: 'button' | 'submit' | 'reset';
}>(), {
  variant: 'primary',
  size: 'md',
  loading: false,
  disabled: false,
  type: 'button',
});

const emit = defineEmits<{
  (e: 'click', event: MouseEvent): void
}>();

const classes = computed(() => {
  const base = 'inline-flex items-center justify-center font-medium transition-all duration-220 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed';

  const variants = {
    primary: 'bg-primary-500 text-white border border-primary-600/50 shadow-fabric hover:bg-primary-600 hover:shadow-fabric-lg hover:-translate-y-0.5 active:translate-y-0 active:bg-primary-700 focus:ring-primary-400',
    secondary: 'bg-fabric-canvas text-fabric-sepia border border-fabric-sand/50 shadow-fabric-inner hover:bg-fabric-sand/30 hover:shadow-fabric active:translate-y-px active:shadow-fabric-inner focus:ring-fabric-thread/30',
    danger: 'bg-red-500 text-white border border-red-600/50 shadow-fabric hover:bg-red-600 hover:shadow-fabric-lg hover:-translate-y-0.5 active:translate-y-0 active:bg-red-700 focus:ring-red-400',
    ghost: 'text-fabric-thread hover:bg-fabric-sand/30 hover:text-fabric-sepia focus:ring-fabric-thread/30',
  };

  const sizes = {
    sm: 'px-3 py-1.5 text-xs rounded-md',
    md: 'px-4 py-2 text-sm rounded-lg',
    lg: 'px-6 py-3 text-base rounded-xl',
  };

  return [
    base,
    variants[props.variant],
    sizes[props.size],
    props.loading ? 'cursor-wait' : ''
  ].join(' ');
});
</script>

<template>
  <button
    :type="type"
    :class="classes"
    :disabled="disabled || loading"
    @click="emit('click', $event)"
  >
    <svg 
      v-if="loading" 
      class="animate-spin -ml-1 mr-2 h-4 w-4 text-current" 
      xmlns="http://www.w3.org/2000/svg" 
      fill="none" 
      viewBox="0 0 24 24"
    >
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
    </svg>
    <slot />
  </button>
</template>
