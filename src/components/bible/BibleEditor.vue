<script setup lang="ts">
import { ref, computed, watch } from 'vue';

export type EntityType = 'character' | 'setting' | 'event';

export interface CharacterFormData {
  id?: number;
  name: string;
  aliases: string[];
  description: string;
  role: string;
  traits: string[];
  notes: string;
}

export interface SettingFormData {
  id?: number;
  name: string;
  setting_type: string;
  description: string;
  properties: Record<string, string>;
  notes: string;
}

export interface EventFormData {
  id?: number;
  title: string;
  description: string;
  importance: string;
  characters_involved: string[];
  notes: string;
}

export type EntityFormData = CharacterFormData | SettingFormData | EventFormData;

const props = defineProps<{
  visible: boolean;
  entityType: EntityType;
  initialData?: EntityFormData;
  isNew?: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', data: EntityFormData): void;
  (e: 'delete', id: number): void;
}>();

// Form data refs
const characterForm = ref<CharacterFormData>({
  name: '',
  aliases: [],
  description: '',
  role: 'minor',
  traits: [],
  notes: '',
});

const settingForm = ref<SettingFormData>({
  name: '',
  setting_type: 'location',
  description: '',
  properties: {},
  notes: '',
});

const eventForm = ref<EventFormData>({
  title: '',
  description: '',
  importance: 'normal',
  characters_involved: [],
  notes: '',
});

// Temporary inputs for array fields
const newAlias = ref('');
const newTrait = ref('');
const newCharacter = ref('');
const newPropertyKey = ref('');
const newPropertyValue = ref('');

// Initialize form data when props change
watch(
  () => [props.visible, props.initialData],
  () => {
    if (props.visible && props.initialData) {
      if (props.entityType === 'character') {
        const data = props.initialData as CharacterFormData;
        characterForm.value = {
          id: data.id,
          name: data.name || '',
          aliases: [...(data.aliases || [])],
          description: data.description || '',
          role: data.role || 'minor',
          traits: [...(data.traits || [])],
          notes: data.notes || '',
        };
      } else if (props.entityType === 'setting') {
        const data = props.initialData as SettingFormData;
        settingForm.value = {
          id: data.id,
          name: data.name || '',
          setting_type: data.setting_type || 'location',
          description: data.description || '',
          properties: { ...(data.properties || {}) },
          notes: data.notes || '',
        };
      } else if (props.entityType === 'event') {
        const data = props.initialData as EventFormData;
        eventForm.value = {
          id: data.id,
          title: data.title || '',
          description: data.description || '',
          importance: data.importance || 'normal',
          characters_involved: [...(data.characters_involved || [])],
          notes: data.notes || '',
        };
      }
    } else if (props.visible && !props.initialData) {
      // Reset forms for new entry
      resetForms();
    }
  },
  { immediate: true }
);

function resetForms() {
  characterForm.value = {
    name: '',
    aliases: [],
    description: '',
    role: 'minor',
    traits: [],
    notes: '',
  };
  settingForm.value = {
    name: '',
    setting_type: 'location',
    description: '',
    properties: {},
    notes: '',
  };
  eventForm.value = {
    title: '',
    description: '',
    importance: 'normal',
    characters_involved: [],
    notes: '',
  };
  newAlias.value = '';
  newTrait.value = '';
  newCharacter.value = '';
  newPropertyKey.value = '';
  newPropertyValue.value = '';
}

const title = computed(() => {
  const action = props.isNew ? '新建' : '编辑';
  const typeMap: Record<EntityType, string> = {
    character: '人物',
    setting: '设定',
    event: '事件',
  };
  return `${action}${typeMap[props.entityType]}`;
});

const roleOptions = [
  { value: 'protagonist', label: '主角' },
  { value: 'antagonist', label: '反派' },
  { value: 'major', label: '主要角色' },
  { value: 'supporting', label: '配角' },
  { value: 'minor', label: '龙套' },
];

const settingTypeOptions = [
  { value: 'location', label: '地点' },
  { value: 'organization', label: '组织' },
  { value: 'item', label: '物品' },
  { value: 'concept', label: '概念' },
  { value: 'custom', label: '自定义' },
];

const importanceOptions = [
  { value: 'critical', label: '关键' },
  { value: 'major', label: '重要' },
  { value: 'normal', label: '普通' },
  { value: 'minor', label: '次要' },
];

// Array field helpers
function addAlias() {
  const val = newAlias.value.trim();
  if (val && !characterForm.value.aliases.includes(val)) {
    characterForm.value.aliases.push(val);
    newAlias.value = '';
  }
}

function removeAlias(index: number) {
  characterForm.value.aliases.splice(index, 1);
}

function addTrait() {
  const val = newTrait.value.trim();
  if (val && !characterForm.value.traits.includes(val)) {
    characterForm.value.traits.push(val);
    newTrait.value = '';
  }
}

function removeTrait(index: number) {
  characterForm.value.traits.splice(index, 1);
}

function addCharacter() {
  const val = newCharacter.value.trim();
  if (val && !eventForm.value.characters_involved.includes(val)) {
    eventForm.value.characters_involved.push(val);
    newCharacter.value = '';
  }
}

function removeCharacter(index: number) {
  eventForm.value.characters_involved.splice(index, 1);
}

function addProperty() {
  const key = newPropertyKey.value.trim();
  const value = newPropertyValue.value.trim();
  if (key && value) {
    settingForm.value.properties[key] = value;
    newPropertyKey.value = '';
    newPropertyValue.value = '';
  }
}

function removeProperty(key: string) {
  delete settingForm.value.properties[key];
}

function handleSave() {
  let data: EntityFormData;
  if (props.entityType === 'character') {
    data = { ...characterForm.value };
  } else if (props.entityType === 'setting') {
    data = { ...settingForm.value };
  } else {
    data = { ...eventForm.value };
  }
  emit('save', data);
}

function handleDelete() {
  const id = props.initialData && 'id' in props.initialData ? props.initialData.id : undefined;
  if (id !== undefined) {
    emit('delete', id);
  }
}

function handleClose() {
  emit('close');
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="visible"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
        @click.self="handleClose"
      >
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/50 backdrop-blur-sm"></div>

        <!-- Modal -->
        <div
          class="relative bg-white rounded-2xl shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col overflow-hidden"
        >
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-gray-100">
            <h3 class="text-lg font-bold text-gray-900">{{ title }}</h3>
            <button
              @click="handleClose"
              class="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-5 w-5"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fill-rule="evenodd"
                  d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                  clip-rule="evenodd"
                />
              </svg>
            </button>
          </div>

          <!-- Content -->
          <div class="flex-1 overflow-y-auto px-6 py-4">
            <!-- Character Form -->
            <div v-if="entityType === 'character'" class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">名称 *</label>
                <input
                  v-model="characterForm.name"
                  type="text"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-character focus:border-transparent"
                  placeholder="角色名称"
                />
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">角色类型</label>
                <select
                  v-model="characterForm.role"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-character focus:border-transparent"
                >
                  <option v-for="opt in roleOptions" :key="opt.value" :value="opt.value">
                    {{ opt.label }}
                  </option>
                </select>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">别名</label>
                <div class="flex gap-2 mb-2">
                  <input
                    v-model="newAlias"
                    type="text"
                    class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-character focus:border-transparent"
                    placeholder="添加别名"
                    @keyup.enter="addAlias"
                  />
                  <button
                    @click="addAlias"
                    class="px-3 py-2 bg-accent-character/10 text-accent-character rounded-lg hover:bg-accent-character/20 transition-colors"
                  >
                    添加
                  </button>
                </div>
                <div class="flex flex-wrap gap-2">
                  <span
                    v-for="(alias, index) in characterForm.aliases"
                    :key="index"
                    class="inline-flex items-center gap-1 px-2 py-1 bg-gray-100 text-gray-700 rounded-lg text-sm"
                  >
                    {{ alias }}
                    <button @click="removeAlias(index)" class="text-gray-400 hover:text-red-500">
                      x
                    </button>
                  </span>
                </div>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">描述</label>
                <textarea
                  v-model="characterForm.description"
                  rows="4"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-character focus:border-transparent resize-none"
                  placeholder="角色描述..."
                ></textarea>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">特征</label>
                <div class="flex gap-2 mb-2">
                  <input
                    v-model="newTrait"
                    type="text"
                    class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-character focus:border-transparent"
                    placeholder="添加特征"
                    @keyup.enter="addTrait"
                  />
                  <button
                    @click="addTrait"
                    class="px-3 py-2 bg-accent-character/10 text-accent-character rounded-lg hover:bg-accent-character/20 transition-colors"
                  >
                    添加
                  </button>
                </div>
                <div class="flex flex-wrap gap-2">
                  <span
                    v-for="(trait, index) in characterForm.traits"
                    :key="index"
                    class="inline-flex items-center gap-1 px-2 py-1 bg-accent-character/10 text-accent-character rounded-lg text-sm"
                  >
                    {{ trait }}
                    <button @click="removeTrait(index)" class="hover:text-red-500">x</button>
                  </span>
                </div>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">备注</label>
                <textarea
                  v-model="characterForm.notes"
                  rows="2"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-character focus:border-transparent resize-none"
                  placeholder="其他备注..."
                ></textarea>
              </div>
            </div>

            <!-- Setting Form -->
            <div v-if="entityType === 'setting'" class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">名称 *</label>
                <input
                  v-model="settingForm.name"
                  type="text"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-setting focus:border-transparent"
                  placeholder="设定名称"
                />
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">类型</label>
                <select
                  v-model="settingForm.setting_type"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-setting focus:border-transparent"
                >
                  <option v-for="opt in settingTypeOptions" :key="opt.value" :value="opt.value">
                    {{ opt.label }}
                  </option>
                </select>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">描述</label>
                <textarea
                  v-model="settingForm.description"
                  rows="4"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-setting focus:border-transparent resize-none"
                  placeholder="详细描述..."
                ></textarea>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">属性</label>
                <div class="flex gap-2 mb-2">
                  <input
                    v-model="newPropertyKey"
                    type="text"
                    class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-setting focus:border-transparent"
                    placeholder="属性名"
                  />
                  <input
                    v-model="newPropertyValue"
                    type="text"
                    class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-setting focus:border-transparent"
                    placeholder="属性值"
                    @keyup.enter="addProperty"
                  />
                  <button
                    @click="addProperty"
                    class="px-3 py-2 bg-accent-setting/10 text-accent-setting rounded-lg hover:bg-accent-setting/20 transition-colors"
                  >
                    添加
                  </button>
                </div>
                <div class="space-y-1">
                  <div
                    v-for="(value, key) in settingForm.properties"
                    :key="key"
                    class="flex items-center justify-between px-3 py-2 bg-gray-50 rounded-lg"
                  >
                    <span class="text-sm">
                      <span class="font-medium text-gray-700">{{ key }}:</span>
                      <span class="text-gray-600 ml-2">{{ value }}</span>
                    </span>
                    <button
                      @click="removeProperty(key as string)"
                      class="text-gray-400 hover:text-red-500"
                    >
                      x
                    </button>
                  </div>
                </div>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">备注</label>
                <textarea
                  v-model="settingForm.notes"
                  rows="2"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-setting focus:border-transparent resize-none"
                  placeholder="其他备注..."
                ></textarea>
              </div>
            </div>

            <!-- Event Form -->
            <div v-if="entityType === 'event'" class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">标题 *</label>
                <input
                  v-model="eventForm.title"
                  type="text"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-event focus:border-transparent"
                  placeholder="事件标题"
                />
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">重要程度</label>
                <select
                  v-model="eventForm.importance"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-event focus:border-transparent"
                >
                  <option v-for="opt in importanceOptions" :key="opt.value" :value="opt.value">
                    {{ opt.label }}
                  </option>
                </select>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">描述</label>
                <textarea
                  v-model="eventForm.description"
                  rows="4"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-event focus:border-transparent resize-none"
                  placeholder="事件描述..."
                ></textarea>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">相关人物</label>
                <div class="flex gap-2 mb-2">
                  <input
                    v-model="newCharacter"
                    type="text"
                    class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-event focus:border-transparent"
                    placeholder="添加相关人物"
                    @keyup.enter="addCharacter"
                  />
                  <button
                    @click="addCharacter"
                    class="px-3 py-2 bg-accent-event/10 text-accent-event rounded-lg hover:bg-accent-event/20 transition-colors"
                  >
                    添加
                  </button>
                </div>
                <div class="flex flex-wrap gap-2">
                  <span
                    v-for="(char, index) in eventForm.characters_involved"
                    :key="index"
                    class="inline-flex items-center gap-1 px-2 py-1 bg-accent-event/10 text-accent-event rounded-lg text-sm"
                  >
                    {{ char }}
                    <button @click="removeCharacter(index)" class="hover:text-red-500">x</button>
                  </span>
                </div>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">备注</label>
                <textarea
                  v-model="eventForm.notes"
                  rows="2"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-accent-event focus:border-transparent resize-none"
                  placeholder="其他备注..."
                ></textarea>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-between px-6 py-4 border-t border-gray-100 bg-gray-50">
            <div>
              <button
                v-if="!isNew && initialData && 'id' in initialData"
                @click="handleDelete"
                class="px-4 py-2 text-red-600 hover:bg-red-50 rounded-lg transition-colors text-sm font-medium"
              >
                删除
              </button>
            </div>
            <div class="flex gap-3">
              <button
                @click="handleClose"
                class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg transition-colors text-sm font-medium"
              >
                取消
              </button>
              <button
                @click="handleSave"
                class="px-6 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors text-sm font-medium shadow-sm"
              >
                保存
              </button>
            </div>
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
