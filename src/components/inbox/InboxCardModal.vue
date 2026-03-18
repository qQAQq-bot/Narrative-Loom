<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface InboxItem {
  id: string;
  chapter_id: string;
  knowledge_type: string;
  title: string;
  content: Record<string, unknown>;
  evidence: string[];
  confidence: string;
  created_at: string;
}

interface MergeCandidate {
  id: string;
  name: string;
  type: string;
  description?: string;
  setting_type?: string;
}

const props = defineProps<{
  visible: boolean;
  bookId: string;
  item: InboxItem | null;
  mode: 'edit' | 'merge';
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'saved'): void;
}>();

// Edit mode state
const editedContent = ref<Record<string, unknown>>({});
const isSubmitting = ref(false);
const error = ref<string | null>(null);

// Merge mode state
const mergeCandidates = ref<MergeCandidate[]>([]);
const selectedTargetId = ref<string | null>(null);
const mergeStrategy = ref<'append' | 'replace'>('append');
const isLoadingCandidates = ref(false);

// Initialize form data when item changes
watch(
  () => [props.visible, props.item],
  async () => {
    if (props.visible && props.item) {
      // Initialize edit content
      editedContent.value = { ...props.item.content };

      // For events, ensure title is in content
      if (props.item.knowledge_type === 'event' && !editedContent.value.title) {
        editedContent.value.title = props.item.title;
      }

      // Load merge candidates if in merge mode
      if (props.mode === 'merge') {
        await loadMergeCandidates();
      }
    }
  },
  { immediate: true }
);

async function loadMergeCandidates() {
  if (!props.item) return;

  isLoadingCandidates.value = true;
  error.value = null;

  try {
    mergeCandidates.value = await invoke<MergeCandidate[]>('get_merge_candidates', {
      bookId: props.bookId,
      cardId: props.item.id,
    });
  } catch (e) {
    console.error('Failed to load merge candidates:', e);
    error.value = '加载合并目标失败';
    mergeCandidates.value = [];
  } finally {
    isLoadingCandidates.value = false;
  }
}

async function handleEditSubmit() {
  if (!props.item) return;

  isSubmitting.value = true;
  error.value = null;

  try {
    await invoke('accept_card_with_edits', {
      bookId: props.bookId,
      cardId: props.item.id,
      editedContent: editedContent.value,
    });
    emit('saved');
    emit('close');
  } catch (e) {
    console.error('Failed to accept card with edits:', e);
    error.value = typeof e === 'string' ? e : '保存失败';
  } finally {
    isSubmitting.value = false;
  }
}

async function handleMergeSubmit() {
  if (!props.item || !selectedTargetId.value) return;

  isSubmitting.value = true;
  error.value = null;

  try {
    await invoke('merge_card', {
      bookId: props.bookId,
      cardId: props.item.id,
      targetEntityId: selectedTargetId.value,
      mergeStrategy: mergeStrategy.value,
    });
    emit('saved');
    emit('close');
  } catch (e) {
    console.error('Failed to merge card:', e);
    error.value = typeof e === 'string' ? e : '合并失败';
  } finally {
    isSubmitting.value = false;
  }
}

function handleClose() {
  error.value = null;
  selectedTargetId.value = null;
  emit('close');
}

const typeLabel = computed(() => {
  if (!props.item) return '';
  switch (props.item.knowledge_type) {
    case 'character': return '人物';
    case 'setting': return '设定';
    case 'event': return '事件';
    default: return props.item.knowledge_type;
  }
});

const modalTitle = computed(() => {
  if (props.mode === 'merge') {
    return `合并${typeLabel.value}到已有条目`;
  }
  return `编辑并接受${typeLabel.value}`;
});

// Array field helpers for character
const newAlias = ref('');
const newTrait = ref('');

function addAlias() {
  const val = newAlias.value.trim();
  if (!val) return;
  const aliases = (editedContent.value.aliases as string[]) || [];
  if (!aliases.includes(val)) {
    editedContent.value.aliases = [...aliases, val];
  }
  newAlias.value = '';
}

function removeAlias(index: number) {
  const aliases = (editedContent.value.aliases as string[]) || [];
  editedContent.value.aliases = aliases.filter((_, i) => i !== index);
}

function addTrait() {
  const val = newTrait.value.trim();
  if (!val) return;
  const traits = (editedContent.value.traits as string[]) || [];
  if (!traits.includes(val)) {
    editedContent.value.traits = [...traits, val];
  }
  newTrait.value = '';
}

function removeTrait(index: number) {
  const traits = (editedContent.value.traits as string[]) || [];
  editedContent.value.traits = traits.filter((_, i) => i !== index);
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="visible && item"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
        @click.self="handleClose"
      >
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/50 backdrop-blur-sm"></div>

        <!-- Modal -->
        <div
          class="relative bg-white dark:bg-gray-800 rounded-2xl shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col overflow-hidden"
        >
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-gray-100 dark:border-gray-700">
            <h3 class="text-lg font-bold text-gray-900 dark:text-white">{{ modalTitle }}</h3>
            <button
              @click="handleClose"
              class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
              </svg>
            </button>
          </div>

          <!-- Error -->
          <div v-if="error" class="mx-6 mt-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 px-4 py-2 rounded-lg text-sm">
            {{ error }}
          </div>

          <!-- Content -->
          <div class="flex-1 overflow-y-auto px-6 py-4">
            <!-- Edit Mode -->
            <template v-if="mode === 'edit'">
              <!-- Character Edit Form -->
              <div v-if="item.knowledge_type === 'character'" class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">名称</label>
                  <input
                    v-model="editedContent.name"
                    type="text"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-accent-character focus:border-transparent"
                  />
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">角色类型</label>
                  <select
                    v-model="editedContent.role"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-accent-character focus:border-transparent"
                  >
                    <option value="protagonist">主角</option>
                    <option value="antagonist">反派</option>
                    <option value="major">主要角色</option>
                    <option value="supporting">配角</option>
                    <option value="minor">龙套</option>
                  </select>
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">别名</label>
                  <div class="flex gap-2 mb-2">
                    <input
                      v-model="newAlias"
                      type="text"
                      class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                      placeholder="添加别名"
                      @keyup.enter="addAlias"
                    />
                    <button @click="addAlias" class="px-3 py-2 bg-accent-character/10 text-accent-character rounded-lg hover:bg-accent-character/20">添加</button>
                  </div>
                  <div class="flex flex-wrap gap-2">
                    <span
                      v-for="(alias, index) in (editedContent.aliases as string[] || [])"
                      :key="index"
                      class="inline-flex items-center gap-1 px-2 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg text-sm"
                    >
                      {{ alias }}
                      <button @click="removeAlias(index)" class="text-gray-400 hover:text-red-500">x</button>
                    </span>
                  </div>
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">描述</label>
                  <textarea
                    v-model="editedContent.description"
                    rows="4"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white resize-none"
                  ></textarea>
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">特征</label>
                  <div class="flex gap-2 mb-2">
                    <input
                      v-model="newTrait"
                      type="text"
                      class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                      placeholder="添加特征"
                      @keyup.enter="addTrait"
                    />
                    <button @click="addTrait" class="px-3 py-2 bg-accent-character/10 text-accent-character rounded-lg hover:bg-accent-character/20">添加</button>
                  </div>
                  <div class="flex flex-wrap gap-2">
                    <span
                      v-for="(trait, index) in (editedContent.traits as string[] || [])"
                      :key="index"
                      class="inline-flex items-center gap-1 px-2 py-1 bg-accent-character/10 text-accent-character rounded-lg text-sm"
                    >
                      {{ trait }}
                      <button @click="removeTrait(index)" class="hover:text-red-500">x</button>
                    </span>
                  </div>
                </div>
              </div>

              <!-- Setting Edit Form -->
              <div v-else-if="item.knowledge_type === 'setting'" class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">名称</label>
                  <input
                    v-model="editedContent.name"
                    type="text"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-accent-setting focus:border-transparent"
                  />
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">类型</label>
                  <select
                    v-model="editedContent.type"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                  >
                    <option value="location">地点</option>
                    <option value="organization">组织</option>
                    <option value="item">物品</option>
                    <option value="concept">概念</option>
                    <option value="custom">自定义</option>
                  </select>
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">描述</label>
                  <textarea
                    v-model="editedContent.description"
                    rows="4"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white resize-none"
                  ></textarea>
                </div>
              </div>

              <!-- Event Edit Form -->
              <div v-else-if="item.knowledge_type === 'event'" class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">标题</label>
                  <input
                    v-model="editedContent.title"
                    type="text"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-accent-event focus:border-transparent"
                  />
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">重要程度</label>
                  <select
                    v-model="editedContent.importance"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                  >
                    <option value="critical">关键</option>
                    <option value="major">重要</option>
                    <option value="normal">普通</option>
                    <option value="minor">次要</option>
                  </select>
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">描述</label>
                  <textarea
                    v-model="editedContent.description"
                    rows="4"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white resize-none"
                  ></textarea>
                </div>
              </div>
            </template>

            <!-- Merge Mode -->
            <template v-else-if="mode === 'merge'">
              <div v-if="isLoadingCandidates" class="flex justify-center py-8">
                <div class="animate-spin rounded-full h-8 w-8 border-2 border-gray-300 dark:border-gray-600 border-t-primary-600"></div>
              </div>

              <div v-else-if="mergeCandidates.length === 0" class="text-center py-8 text-gray-500 dark:text-gray-400">
                <p>没有可合并的目标条目</p>
                <p class="text-sm mt-1">请先在故事圣经中创建相应的条目</p>
              </div>

              <div v-else class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">选择要合并到的目标</label>
                  <div class="space-y-2 max-h-60 overflow-y-auto">
                    <label
                      v-for="candidate in mergeCandidates"
                      :key="candidate.id"
                      class="flex items-start gap-3 p-3 border rounded-lg cursor-pointer transition-colors"
                      :class="selectedTargetId === candidate.id
                        ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20'
                        : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'"
                    >
                      <input
                        type="radio"
                        :value="candidate.id"
                        v-model="selectedTargetId"
                        class="mt-1"
                      />
                      <div class="flex-1 min-w-0">
                        <div class="font-medium text-gray-900 dark:text-white">{{ candidate.name }}</div>
                        <div v-if="candidate.description" class="text-sm text-gray-500 dark:text-gray-400 line-clamp-2 mt-1">
                          {{ candidate.description }}
                        </div>
                      </div>
                    </label>
                  </div>
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">合并策略</label>
                  <div class="flex gap-4">
                    <label class="flex items-center gap-2">
                      <input type="radio" value="append" v-model="mergeStrategy" />
                      <span class="text-sm text-gray-700 dark:text-gray-300">追加内容</span>
                    </label>
                    <label class="flex items-center gap-2">
                      <input type="radio" value="replace" v-model="mergeStrategy" />
                      <span class="text-sm text-gray-700 dark:text-gray-300">替换描述</span>
                    </label>
                  </div>
                  <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                    追加：新内容会添加到现有描述后面；替换：新描述会覆盖现有描述
                  </p>
                </div>
              </div>
            </template>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-gray-100 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
            <button
              @click="handleClose"
              class="px-4 py-2 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors text-sm font-medium"
            >
              取消
            </button>
            <button
              v-if="mode === 'edit'"
              @click="handleEditSubmit"
              :disabled="isSubmitting"
              class="px-6 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors text-sm font-medium shadow-sm disabled:opacity-50 flex items-center gap-2"
            >
              <span v-if="isSubmitting" class="animate-spin">
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
              </span>
              保存并接受
            </button>
            <button
              v-else
              @click="handleMergeSubmit"
              :disabled="isSubmitting || !selectedTargetId"
              class="px-6 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors text-sm font-medium shadow-sm disabled:opacity-50 flex items-center gap-2"
            >
              <span v-if="isSubmitting" class="animate-spin">
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
              </span>
              合并
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
  transition: all 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .relative,
.modal-leave-to .relative {
  transform: scale(0.95);
}
</style>
