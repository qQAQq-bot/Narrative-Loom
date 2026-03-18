<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import type { PromptCard, PromptCardPosition } from '@/stores/settings';

const props = defineProps<{
  visible: boolean;
  card?: PromptCard | null;
  isNew?: boolean;
  defaultPosition?: PromptCardPosition;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', card: PromptCard): void;
}>();

const formData = ref<{
  id: string;
  title: string;
  content: string;
  enabled: boolean;
  position: PromptCardPosition;
}>({
  id: '',
  title: '',
  content: '',
  enabled: true,
  position: 'prefix',
});

const contentCharCount = computed(() => formData.value.content.length);
const contentWarning = computed(() => contentCharCount.value > 4000);

const showPositionSelector = computed(() => {
  if (!props.isNew) return true;
  return !props.defaultPosition;
});

const titleError = ref('');
const contentError = ref('');

watch(
  () => [props.visible, props.card],
  () => {
    if (props.visible) {
      titleError.value = '';
      contentError.value = '';

      if (props.card && !props.isNew) {
        formData.value = {
          id: props.card.id,
          title: props.card.title,
          content: props.card.content,
          enabled: props.card.enabled,
          position: props.card.position,
        };
      } else {
        formData.value = {
          id: crypto.randomUUID().substring(0, 8),
          title: '',
          content: '',
          enabled: true,
          position: props.defaultPosition ?? 'prefix',
        };
      }
    }
  },
  { immediate: true }
);

function validate(): boolean {
  let valid = true;
  titleError.value = '';
  contentError.value = '';

  if (!formData.value.title.trim()) {
    titleError.value = '标题不能为空';
    valid = false;
  }

  if (!formData.value.content.trim()) {
    contentError.value = '内容不能为空';
    valid = false;
  }

  return valid;
}

function handleSave() {
  if (!validate()) return;

  emit('save', {
    id: formData.value.id,
    title: formData.value.title.trim(),
    content: formData.value.content.trim(),
    enabled: formData.value.enabled,
    position: formData.value.position,
    order: props.card?.order ?? 0,
    updated_at: new Date().toISOString(),
  });
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
      @click.self="emit('close')"
    >
      <div class="bg-fabric-warm rounded-xl border border-fabric-sand/40 shadow-xl w-full max-w-lg mx-4">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-fabric-sand/30">
          <h3 class="text-lg font-medium text-fabric-sepia">
            {{ isNew ? '新建提示词卡片' : '编辑提示词卡片' }}
          </h3>
          <button
            @click="emit('close')"
            class="text-fabric-thread/50 hover:text-fabric-thread transition-colors"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Body -->
        <div class="px-6 py-4 space-y-4">
          <!-- Title -->
          <div>
            <label class="block text-sm font-medium text-fabric-sepia mb-1">标题</label>
            <input
              v-model="formData.title"
              type="text"
              placeholder="例如：文学分析免责声明"
              class="w-full px-3 py-2 bg-fabric-linen/50 border border-fabric-sand/40 rounded-lg text-sm text-fabric-sepia placeholder-fabric-thread/40 focus:outline-none focus:ring-2 focus:ring-primary-500/30 focus:border-primary-500/50"
            />
            <p v-if="titleError" class="text-xs text-red-500 mt-1">{{ titleError }}</p>
          </div>

          <!-- Position -->
          <div v-if="showPositionSelector">
            <label class="block text-sm font-medium text-fabric-sepia mb-1">位置</label>
            <div class="flex gap-3">
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  v-model="formData.position"
                  type="radio"
                  value="prefix"
                  class="text-primary-500 focus:ring-primary-500/30"
                />
                <span class="text-sm text-fabric-thread">前置 (Prefix)</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  v-model="formData.position"
                  type="radio"
                  value="suffix"
                  class="text-primary-500 focus:ring-primary-500/30"
                />
                <span class="text-sm text-fabric-thread">后置 (Suffix)</span>
              </label>
            </div>
            <p class="text-xs text-fabric-thread/50 mt-1">
              {{ formData.position === 'prefix' ? '将添加在 Agent 系统提示词之前' : '将添加在 Agent 系统提示词之后' }}
            </p>
          </div>

          <!-- Enabled -->
          <div class="flex items-center justify-between">
            <label class="text-sm font-medium text-fabric-sepia">启用</label>
            <button
              @click="formData.enabled = !formData.enabled"
              :class="[
                'relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-200',
                formData.enabled ? 'bg-primary-500' : 'bg-fabric-sand/60'
              ]"
            >
              <span
                :class="[
                  'inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform duration-200',
                  formData.enabled ? 'translate-x-[18px]' : 'translate-x-[3px]'
                ]"
              />
            </button>
          </div>

          <!-- Content -->
          <div>
            <label class="block text-sm font-medium text-fabric-sepia mb-1">内容</label>
            <textarea
              v-model="formData.content"
              rows="6"
              placeholder="输入要注入到系统提示词中的文本..."
              class="w-full px-3 py-2 bg-fabric-linen/50 border border-fabric-sand/40 rounded-lg text-sm text-fabric-sepia placeholder-fabric-thread/40 focus:outline-none focus:ring-2 focus:ring-primary-500/30 focus:border-primary-500/50 resize-y font-mono"
            />
            <div class="flex items-center justify-between mt-1">
              <p v-if="contentError" class="text-xs text-red-500">{{ contentError }}</p>
              <span v-else />
              <span
                :class="['text-xs', contentWarning ? 'text-amber-500' : 'text-fabric-thread/40']"
              >
                {{ contentCharCount }} 字符
                <span v-if="contentWarning">（内容较长，可能影响分析效果）</span>
              </span>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-end gap-2 px-6 py-4 border-t border-fabric-sand/30">
          <button
            @click="emit('close')"
            class="px-4 py-2 text-sm text-fabric-thread/70 hover:bg-fabric-sand/30 rounded-lg transition-colors duration-220"
          >
            取消
          </button>
          <button
            @click="handleSave"
            class="fabric-btn-primary"
          >
            保存
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
