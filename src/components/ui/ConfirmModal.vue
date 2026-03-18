<script setup lang="ts">
import { ref, watch } from 'vue';

const props = withDefaults(defineProps<{
  visible: boolean;
  title?: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  type?: 'danger' | 'warning' | 'info';
  loading?: boolean;
}>(), {
  title: '确认',
  confirmText: '确认',
  cancelText: '取消',
  type: 'danger',
  loading: false,
});

const emit = defineEmits<{
  (e: 'confirm'): void;
  (e: 'cancel'): void;
}>();

function handleConfirm() {
  if (!props.loading) {
    emit('confirm');
  }
}

function handleCancel() {
  if (!props.loading) {
    emit('cancel');
  }
}

const iconColor = {
  danger: 'text-red-500 bg-red-100 dark:bg-red-900/30 dark:text-red-400',
  warning: 'text-amber-500 bg-amber-100 dark:bg-amber-900/30 dark:text-amber-400',
  info: 'text-primary-500 bg-primary-100 dark:bg-primary-900/30 dark:text-primary-400',
};

const buttonColor = {
  danger: 'bg-red-500 hover:bg-red-600 text-white disabled:bg-red-300',
  warning: 'bg-amber-500 hover:bg-amber-600 text-white disabled:bg-amber-300',
  info: 'bg-primary-500 hover:bg-primary-600 text-white disabled:bg-primary-300',
};
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="visible"
        class="fixed inset-0 z-[10000] flex items-center justify-center p-4"
        @click.self="handleCancel"
      >
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-fabric-sepia/40 dark:bg-black/60 backdrop-blur-sm"></div>

        <!-- Modal -->
        <div class="relative bg-fabric-cream dark:bg-fabric-canvas rounded-2xl shadow-fabric-lg w-full max-w-sm overflow-hidden border border-fabric-sand/50 dark:border-fabric-sand/30 transform transition-all duration-200">
          <div class="p-6">
            <!-- Icon -->
            <div class="flex justify-center mb-4">
              <div :class="['w-14 h-14 rounded-full flex items-center justify-center', iconColor[type]]">
                <!-- Danger icon -->
                <svg v-if="type === 'danger'" class="w-7 h-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
                <!-- Warning icon -->
                <svg v-else-if="type === 'warning'" class="w-7 h-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <!-- Info icon -->
                <svg v-else class="w-7 h-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
            </div>

            <!-- Title -->
            <h3 class="text-lg font-semibold text-fabric-sepia text-center mb-2 font-serif">
              {{ title }}
            </h3>

            <!-- Custom content slot -->
            <slot name="content"></slot>

            <!-- Message -->
            <p class="text-fabric-thread/70 text-center text-sm leading-relaxed">
              {{ message }}
            </p>
          </div>

          <!-- Actions -->
          <div class="flex border-t border-fabric-sand/40 dark:border-fabric-sand/30">
            <button
              @click="handleCancel"
              :disabled="loading"
              class="flex-1 px-4 py-3.5 text-fabric-thread/70 dark:text-fabric-thread hover:bg-fabric-sand/20 dark:hover:bg-fabric-sand/10 transition-colors duration-180 text-sm font-medium border-r border-fabric-sand/40 dark:border-fabric-sand/30 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {{ cancelText }}
            </button>
            <button
              @click="handleConfirm"
              :disabled="loading"
              :class="['flex-1 px-4 py-3.5 transition-all duration-180 text-sm font-medium flex items-center justify-center gap-2', buttonColor[type]]"
            >
              <!-- Loading spinner -->
              <svg v-if="loading" class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {{ confirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: all 0.25s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active .relative,
.modal-leave-active .relative {
  transition: all 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.modal-enter-from .relative,
.modal-leave-to .relative {
  transform: scale(0.9) translateY(10px);
  opacity: 0;
}
</style>
