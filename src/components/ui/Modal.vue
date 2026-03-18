<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';

const props = withDefaults(defineProps<{
  show: boolean;
  title?: string;
  maxWidth?: 'sm' | 'md' | 'lg' | 'xl' | '2xl';
}>(), {
  show: false,
  maxWidth: '2xl',
});

const emit = defineEmits<{
  (e: 'close'): void
}>();

const close = () => {
  emit('close');
};

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && props.show) {
    close();
  }
};

onMounted(() => {
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown);
});

const maxWidthClass = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
  xl: 'max-w-xl',
  '2xl': 'max-w-2xl',
};
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="show"
        class="fixed inset-0 z-50 flex items-center justify-center overflow-y-auto overflow-x-hidden p-4 sm:p-6 bg-fabric-sepia/30 dark:bg-black/50 backdrop-blur-sm"
        @click="close"
      >
        <div
          class="relative w-full bg-fabric-cream dark:bg-fabric-canvas rounded-2xl shadow-fabric-lg ring-1 ring-fabric-sand/30 transform transition-all flex flex-col max-h-[90vh]"
          :class="maxWidthClass[maxWidth]"
          @click.stop
        >
          <!-- Header -->
          <div v-if="title || $slots.header" class="flex items-center justify-between px-6 py-4 border-b border-fabric-sand/40">
            <h3 v-if="title" class="text-lg font-serif font-bold text-fabric-sepia">
              {{ title }}
            </h3>
            <slot name="header" />
            <button
              @click="close"
              class="text-fabric-thread/60 hover:text-fabric-sepia transition-colors p-1 rounded-full hover:bg-fabric-sand/30"
            >
              <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Body -->
          <div class="px-6 py-4 overflow-y-auto">
            <slot />
          </div>

          <!-- Footer -->
          <div v-if="$slots.footer" class="px-6 py-4 border-t border-fabric-sand/40 bg-fabric-linen/50 rounded-b-2xl flex justify-end gap-3">
            <slot name="footer" />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
