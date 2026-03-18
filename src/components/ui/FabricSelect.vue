<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue';

interface Option {
  value: string;
  label: string;
  group?: string;
}

interface OptionGroup {
  label: string;
  options: Option[];
}

const props = withDefaults(defineProps<{
  modelValue?: string | number | null;
  options: (Option | OptionGroup)[];
  placeholder?: string;
  disabled?: boolean;
  size?: 'sm' | 'md';
  searchable?: boolean;
  searchPlaceholder?: string;
}>(), {
  modelValue: '',
  placeholder: '请选择',
  disabled: false,
  size: 'md',
  searchable: false,
  searchPlaceholder: '搜索...',
});

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
}>();

const isOpen = ref(false);
const selectRef = ref<HTMLDivElement | null>(null);
const dropdownRef = ref<HTMLDivElement | null>(null);
const searchInputRef = ref<HTMLInputElement | null>(null);
const searchQuery = ref('');

// Dropdown position for teleported element
const dropdownStyle = ref({
  top: '0px',
  left: '0px',
  width: '0px',
});

// Flatten options for easier lookup
const flatOptions = computed(() => {
  const result: Option[] = [];
  for (const item of props.options) {
    if ('options' in item) {
      result.push(...item.options);
    } else {
      result.push(item);
    }
  }
  return result;
});

// Check if options contain groups
const hasGroups = computed(() => {
  return props.options.some(item => 'options' in item);
});

const normalizedModelValue = computed(() => {
  if (props.modelValue === null || props.modelValue === undefined) return '';
  return String(props.modelValue);
});

// Get display label for current value
const displayLabel = computed(() => {
  const option = flatOptions.value.find(o => o.value === normalizedModelValue.value);
  return option?.label || props.placeholder;
});

// Filter options based on search query
const filteredOptions = computed(() => {
  if (!props.searchable || !searchQuery.value.trim()) {
    return props.options;
  }

  const query = searchQuery.value.toLowerCase().trim();

  if (hasGroups.value) {
    // Filter grouped options
    const result: (Option | OptionGroup)[] = [];
    for (const item of props.options) {
      if ('options' in item) {
        const filteredGroupOptions = item.options.filter(opt =>
          opt.label.toLowerCase().includes(query) ||
          opt.value.toLowerCase().includes(query)
        );
        if (filteredGroupOptions.length > 0) {
          result.push({
            label: item.label,
            options: filteredGroupOptions,
          });
        }
      } else {
        if (item.label.toLowerCase().includes(query) ||
            item.value.toLowerCase().includes(query)) {
          result.push(item);
        }
      }
    }
    return result;
  } else {
    // Filter flat options
    return (props.options as Option[]).filter(opt =>
      opt.label.toLowerCase().includes(query) ||
      opt.value.toLowerCase().includes(query)
    );
  }
});

// Check if filtered options have groups
const filteredHasGroups = computed(() => {
  return filteredOptions.value.some(item => 'options' in item);
});

// Check if there are any results
const hasResults = computed(() => {
  if (filteredHasGroups.value) {
    return filteredOptions.value.some(item => {
      if ('options' in item) {
        return item.options.length > 0;
      }
      return true;
    });
  }
  return filteredOptions.value.length > 0;
});

// Update dropdown position
function updateDropdownPosition() {
  if (!selectRef.value) return;

  const rect = selectRef.value.getBoundingClientRect();
  const viewportHeight = window.innerHeight;
  const gap = 4;
  const spaceBelow = viewportHeight - rect.bottom;
  const spaceAbove = rect.top;

  const dropdownMaxHeight = 280;
  const measuredHeight = dropdownRef.value?.getBoundingClientRect().height ?? 0;
  const dropdownHeight = measuredHeight > 0 ? Math.min(measuredHeight, dropdownMaxHeight) : dropdownMaxHeight;

  const showAbove = measuredHeight > 0
    ? spaceBelow < dropdownHeight + gap && spaceAbove > spaceBelow
    : false;

  const top = showAbove
    ? Math.max(gap, rect.top - dropdownHeight - gap)
    : rect.bottom + gap;

  dropdownStyle.value = {
    top: `${top}px`,
    left: `${rect.left}px`,
    width: `${rect.width}px`,
  };
}

// Toggle dropdown
async function toggleDropdown() {
  if (props.disabled) return;
  isOpen.value = !isOpen.value;

  if (isOpen.value) {
    searchQuery.value = '';
    await nextTick();
    updateDropdownPosition();
    await nextTick();
    updateDropdownPosition();
    // Focus search input if searchable
    if (props.searchable && searchInputRef.value) {
      searchInputRef.value.focus();
    }
  }
}

// Select an option
function selectOption(value: string) {
  emit('update:modelValue', value);
  isOpen.value = false;
  searchQuery.value = '';
}

// Close dropdown when clicking outside
function handleClickOutside(event: MouseEvent) {
  const target = event.target as Node;
  if (selectRef.value && !selectRef.value.contains(target)) {
    if (dropdownRef.value && !dropdownRef.value.contains(target)) {
      isOpen.value = false;
      searchQuery.value = '';
    }
  }
}

// Close on escape key
function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    isOpen.value = false;
    searchQuery.value = '';
  }
}

// Update position on scroll/resize
function handleScrollResize() {
  if (isOpen.value) {
    updateDropdownPosition();
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
  document.addEventListener('keydown', handleKeydown);
  window.addEventListener('scroll', handleScrollResize, true);
  window.addEventListener('resize', handleScrollResize);
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  document.removeEventListener('keydown', handleKeydown);
  window.removeEventListener('scroll', handleScrollResize, true);
  window.removeEventListener('resize', handleScrollResize);
});

const sizeClasses = computed(() => {
  return props.size === 'sm'
    ? 'px-3 py-1.5 text-sm'
    : 'px-3 py-2';
});

const dropdownSizeClasses = computed(() => {
  return props.size === 'sm'
    ? 'text-sm'
    : '';
});
</script>

<template>
  <div ref="selectRef" class="relative">
    <!-- Select Button -->
    <button
      type="button"
      @click="toggleDropdown"
      :disabled="disabled"
      :class="[
        'w-full flex items-center justify-between gap-2 border border-fabric-sand/50 rounded-lg',
        'bg-fabric-warm text-fabric-sepia',
        'focus:ring-2 focus:ring-primary-400/50 focus:border-primary-400',
        'transition-colors duration-180',
        'disabled:bg-fabric-linen/50 disabled:text-fabric-thread/50 disabled:cursor-not-allowed',
        sizeClasses,
        isOpen ? 'ring-2 ring-primary-400/50 border-primary-400' : '',
      ]"
    >
      <span
        :class="[
          'flex-1 min-w-0 truncate text-left',
          normalizedModelValue ? 'text-fabric-sepia' : 'text-fabric-thread/40',
        ]"
      >
        {{ displayLabel }}
      </span>
      <svg
        :class="[
          'w-4 h-4 shrink-0 text-fabric-thread/60 transition-transform duration-180',
          isOpen ? 'rotate-180' : '',
        ]"
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
      </svg>
    </button>

    <!-- Dropdown (Teleported to body to avoid overflow clipping) -->
    <Teleport to="body">
      <Transition
        enter-active-class="transition duration-150 ease-out"
        enter-from-class="opacity-0 scale-95"
        enter-to-class="opacity-100 scale-100"
        leave-active-class="transition duration-100 ease-in"
        leave-from-class="opacity-100 scale-100"
        leave-to-class="opacity-0 scale-95"
      >
        <div
          v-if="isOpen"
          ref="dropdownRef"
          :style="dropdownStyle"
          :class="[
            'fixed z-[9999] bg-fabric-cream border border-fabric-sand/50 rounded-lg shadow-fabric-lg',
            'flex flex-col',
            dropdownSizeClasses,
          ]"
          style="max-height: 280px;"
        >
          <!-- Search Input -->
          <div v-if="searchable" class="p-2 border-b border-fabric-sand/30 shrink-0">
            <div class="relative">
              <svg
                class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-fabric-thread/40"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              <input
                ref="searchInputRef"
                v-model="searchQuery"
                type="text"
                :placeholder="searchPlaceholder"
                class="w-full pl-8 pr-3 py-1.5 text-sm border border-fabric-sand/40 rounded-md bg-fabric-warm text-fabric-sepia placeholder:text-fabric-thread/40 focus:outline-none focus:ring-1 focus:ring-primary-400/50 focus:border-primary-400"
                @click.stop
              />
            </div>
          </div>

          <!-- Options List -->
          <div class="overflow-auto flex-1" style="max-height: 220px;">
            <!-- No Results -->
            <div
              v-if="!hasResults"
              class="px-3 py-4 text-center text-fabric-thread/50 text-sm"
            >
              未找到匹配项
            </div>

            <!-- Grouped Options -->
            <template v-else-if="filteredHasGroups">
              <template v-for="(item, index) in filteredOptions" :key="index">
                <template v-if="'options' in item">
                  <!-- Group Label -->
                  <div class="px-3 py-1.5 text-xs font-medium text-fabric-thread/60 bg-fabric-linen/50 sticky top-0">
                    {{ item.label }}
                  </div>
                  <!-- Group Options -->
                  <button
                    v-for="option in item.options"
                    :key="option.value"
                    type="button"
                    @click="selectOption(option.value)"
                    :class="[
                      'w-full px-3 py-2 text-left transition-colors duration-150',
                      'hover:bg-primary-500/10',
                      normalizedModelValue === option.value
                        ? 'bg-primary-500/15 text-primary-700 dark:text-primary-300'
                        : 'text-fabric-sepia',
                    ]"
                  >
                    {{ option.label }}
                  </button>
                </template>
                <template v-else>
                  <!-- Single Option -->
                  <button
                    type="button"
                    @click="selectOption(item.value)"
                    :class="[
                      'w-full px-3 py-2 text-left transition-colors duration-150',
                      'hover:bg-primary-500/10',
                      normalizedModelValue === item.value
                        ? 'bg-primary-500/15 text-primary-700 dark:text-primary-300'
                        : 'text-fabric-sepia',
                    ]"
                  >
                    {{ item.label }}
                  </button>
                </template>
              </template>
            </template>

            <!-- Flat Options -->
            <template v-else>
              <button
                v-for="option in (filteredOptions as Option[])"
                :key="option.value"
                type="button"
                @click="selectOption(option.value)"
                :class="[
                  'w-full px-3 py-2 text-left transition-colors duration-150',
                  'hover:bg-primary-500/10',
                  normalizedModelValue === option.value
                    ? 'bg-primary-500/15 text-primary-700 dark:text-primary-300'
                    : 'text-fabric-sepia',
                ]"
              >
                {{ option.label }}
              </button>
            </template>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
