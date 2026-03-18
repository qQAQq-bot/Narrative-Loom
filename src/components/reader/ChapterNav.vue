<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue';
import type { ChapterListItem } from '@/stores/book';
import VirtualList from '@/components/common/VirtualList.vue';

// VirtualList exposed methods interface
interface VirtualListExposed {
  scrollToIndex: (index: number, behavior?: ScrollBehavior) => void;
  scrollToTop: (behavior?: ScrollBehavior) => void;
  scrollToBottom: (behavior?: ScrollBehavior) => void;
  updateItemHeight: (index: number, height: number) => void;
}

const virtualListRef = ref<VirtualListExposed | null>(null);

const props = defineProps<{
  chapters: ChapterListItem[];
  currentChapterId: string | null;
  batchMode?: boolean;
  selectedChapterIds?: Set<string>;
}>();

const emit = defineEmits<{
  (e: 'select', chapterId: string): void;
  (e: 'toggleBatchSelect', chapterId: string): void;
  (e: 'selectVolume', volumeTitle: string): void;
}>();

// Track collapsed volumes
const collapsedVolumes = ref<Set<string>>(new Set());

// Check if chapters have volume structure
const hasVolumeStructure = computed(() => {
  return props.chapters.some(c => c.parent_title);
});

// Get all unique parent_titles (volume names)
const volumeNames = computed(() => {
  const names = new Set<string>();
  for (const c of props.chapters) {
    if (c.parent_title) {
      names.add(c.parent_title);
    }
  }
  return names;
});

// Find the "volume chapter" - a chapter whose title matches a volume name and has no parent_title
// This is the chapter that contains the volume's own content
function findVolumeChapter(volumeTitle: string): ChapterListItem | undefined {
  return props.chapters.find(c =>
    c.title === volumeTitle && !c.parent_title
  );
}

// Flatten for virtual list - includes volume headers and visible chapters
interface FlatItem {
  type: 'volume-header' | 'chapter';
  key: string;  // Unique key for VirtualList
  volumeTitle?: string;
  volumeChapter?: ChapterListItem; // The chapter that represents the volume itself (if any)
  chapter?: ChapterListItem;
  chapterCount?: number;
  isExpanded?: boolean;
  isUnderVolume?: boolean;
}

const flatItems = computed((): FlatItem[] => {
  if (!hasVolumeStructure.value) {
    return props.chapters.map(c => ({
      type: 'chapter' as const,
      key: `ch-${c.id}`,
      chapter: c
    }));
  }

  const items: FlatItem[] = [];
  const processedVolumeChapterIds = new Set<string>();

  // First pass: identify chapters that are volume chapters (their title matches a volume name)
  for (const volumeName of volumeNames.value) {
    const volumeChapter = findVolumeChapter(volumeName);
    if (volumeChapter) {
      processedVolumeChapterIds.add(volumeChapter.id);
    }
  }

  // Group chapters by parent_title
  const volumeChildrenMap = new Map<string, ChapterListItem[]>();
  for (const chapter of props.chapters) {
    if (chapter.parent_title) {
      if (!volumeChildrenMap.has(chapter.parent_title)) {
        volumeChildrenMap.set(chapter.parent_title, []);
      }
      volumeChildrenMap.get(chapter.parent_title)!.push(chapter);
    }
  }

  // Build the flat list
  let lastVolume: string | null = null;

  for (const chapter of props.chapters) {
    // Skip chapters that are volume chapters (they will be shown as part of the volume header)
    if (processedVolumeChapterIds.has(chapter.id)) {
      // But we need to add the volume header at this position
      const volumeTitle = chapter.title!;
      const isExpanded = !collapsedVolumes.value.has(volumeTitle);
      const childChapters = volumeChildrenMap.get(volumeTitle) || [];

      items.push({
        type: 'volume-header',
        key: `vol-${volumeTitle}`,
        volumeTitle,
        volumeChapter: chapter, // This volume has its own content
        chapterCount: childChapters.length,
        isExpanded,
      });

      // Add children if expanded
      if (isExpanded) {
        for (const child of childChapters) {
          items.push({
            type: 'chapter',
            key: `ch-${child.id}`,
            chapter: child,
            isUnderVolume: true,
          });
        }
      }

      lastVolume = volumeTitle;
      continue;
    }

    if (chapter.parent_title) {
      // This is a child chapter of a volume
      if (chapter.parent_title !== lastVolume) {
        // New volume that doesn't have its own chapter content
        const volumeTitle: string = chapter.parent_title;
        const isExpanded = !collapsedVolumes.value.has(volumeTitle);
        const childChapters = volumeChildrenMap.get(volumeTitle) || [];

        items.push({
          type: 'volume-header',
          key: `vol-${volumeTitle}`,
          volumeTitle,
          volumeChapter: undefined, // No content for this volume header
          chapterCount: childChapters.length,
          isExpanded,
        });

        // Add children if expanded
        if (isExpanded) {
          for (const child of childChapters) {
            items.push({
              type: 'chapter',
              key: `ch-${child.id}`,
              chapter: child,
              isUnderVolume: true,
            });
          }
        }

        lastVolume = volumeTitle;
      }
      // Skip individual child chapters here as they're added with the volume
    } else {
      // Orphan chapter (no parent_title and not a volume chapter)
      items.push({
        type: 'chapter',
        key: `ch-${chapter.id}`,
        chapter,
        isUnderVolume: false,
      });
      lastVolume = null;
    }
  }

  return items;
});

function toggleVolume(volumeTitle: string, event: Event) {
  event.stopPropagation();
  const newSet = new Set(collapsedVolumes.value);
  if (newSet.has(volumeTitle)) {
    newSet.delete(volumeTitle);
  } else {
    newSet.add(volumeTitle);
  }
  collapsedVolumes.value = newSet;
}

function handleVolumeClick(item: FlatItem) {
  // If volume has its own content, select it
  if (item.volumeChapter) {
    emit('select', item.volumeChapter.id);
  }
}

function getChapterDisplayTitle(chapter: ChapterListItem): string {
  return chapter.title || `第${chapter.index}章`;
}

function formatCharCount(count: number): string {
  if (count >= 10000) {
    return `${(count / 10000).toFixed(1)}万字`;
  }
  return `${count}字`;
}

function isCurrentChapter(chapter: ChapterListItem): boolean {
  return chapter.id === props.currentChapterId;
}

function isCurrentVolumeChapter(item: FlatItem): boolean {
  return item.volumeChapter?.id === props.currentChapterId;
}

function isChapterSelected(chapterId: string): boolean {
  return props.selectedChapterIds?.has(chapterId) ?? false;
}

function handleChapterClick(chapter: ChapterListItem) {
  if (props.batchMode) {
    emit('toggleBatchSelect', chapter.id);
  } else {
    emit('select', chapter.id);
  }
}

// 自动滚动到当前章节
watch(
  () => props.currentChapterId,
  async (newId) => {
    if (newId && virtualListRef.value) {
      await nextTick();
      // Find index in flat items
      const index = flatItems.value.findIndex(
        item => (item.type === 'chapter' && item.chapter?.id === newId) ||
                (item.type === 'volume-header' && item.volumeChapter?.id === newId)
      );
      if (index >= 0) {
        virtualListRef.value.scrollToIndex(index, 'smooth');
      }
    }
  },
  { immediate: true }
);

// Expand volume containing current chapter when it changes
watch(
  () => props.currentChapterId,
  (newId) => {
    if (newId && hasVolumeStructure.value) {
      const chapter = props.chapters.find(c => c.id === newId);
      if (chapter?.parent_title && collapsedVolumes.value.has(chapter.parent_title)) {
        const newSet = new Set(collapsedVolumes.value);
        newSet.delete(chapter.parent_title);
        collapsedVolumes.value = newSet;
      }
    }
  },
  { immediate: true }
);
</script>

<template>
  <div class="chapter-nav">
    <VirtualList
      v-if="flatItems.length > 0"
      ref="virtualListRef"
      :items="flatItems"
      :item-height="36"
      :buffer="5"
      key-field="key"
      class="h-full"
    >
      <template #default="{ item }">
        <!-- Volume Header -->
        <div
          v-if="item.type === 'volume-header'"
          :class="[
            'w-full text-left px-2 py-1.5 rounded-md text-xs transition-colors duration-150 flex items-center gap-1.5 group',
            isCurrentVolumeChapter(item)
              ? 'bg-primary-100 dark:bg-primary-900/30 font-medium'
              : 'bg-fabric-sand/20 hover:bg-fabric-sand/40',
          ]"
        >
          <!-- Expand/Collapse toggle button -->
          <button
            @click="toggleVolume(item.volumeTitle!, $event)"
            class="p-0.5 -ml-0.5 hover:bg-fabric-sand/50 rounded transition-colors shrink-0"
            :title="item.isExpanded ? '收起' : '展开'"
          >
            <svg
              :class="['w-3 h-3 transition-transform duration-200', item.isExpanded ? 'rotate-90' : '']"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
          </button>

          <!-- Volume title (clickable if has content) -->
          <button
            v-if="item.volumeChapter"
            @click="handleVolumeClick(item)"
            :class="[
              'truncate flex-1 text-left',
              isCurrentVolumeChapter(item)
                ? 'text-primary-700 dark:text-primary-300'
                : 'text-fabric-sepia hover:text-fabric-sepia/80',
            ]"
            :title="`${item.volumeTitle} (点击查看内容)`"
          >
            {{ item.volumeTitle }}
          </button>
          <span
            v-else
            class="truncate flex-1 text-fabric-sepia font-medium"
          >
            {{ item.volumeTitle }}
          </span>

          <!-- Chapter count & char count for volume -->
          <span class="flex items-center gap-1 shrink-0">
            <span
              v-if="item.volumeChapter"
              :class="[
                'text-[10px] font-mono',
                isCurrentVolumeChapter(item) ? 'text-primary-500' : 'text-fabric-thread/40'
              ]"
            >
              {{ formatCharCount(item.volumeChapter.char_count) }}
            </span>
            <span class="text-[10px] font-normal text-fabric-thread/50">
              {{ item.chapterCount }}章
            </span>
          </span>
        </div>

        <!-- Chapter Item -->
        <button
          v-else
          @click="handleChapterClick(item.chapter!)"
          :class="[
            'w-full text-left py-1.5 rounded-md text-xs transition-colors duration-150',
            'flex items-center justify-between gap-1.5 group',
            item.isUnderVolume ? 'pl-5 pr-2.5' : 'px-2.5',
            batchMode && isChapterSelected(item.chapter!.id)
              ? 'bg-primary-100 dark:bg-primary-900/30 ring-1 ring-primary-400'
              : isCurrentChapter(item.chapter!)
                ? 'bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 font-medium'
                : 'text-fabric-thread hover:bg-fabric-sand/30',
          ]"
        >
          <!-- Checkbox in batch mode -->
          <span v-if="batchMode" class="shrink-0 mr-1">
            <input
              type="checkbox"
              :checked="isChapterSelected(item.chapter!.id)"
              class="w-3.5 h-3.5 rounded border-fabric-sand/50 text-primary-500 focus:ring-primary-500/30"
              @click.stop
              @change="emit('toggleBatchSelect', item.chapter!.id)"
            />
          </span>

          <span class="truncate flex-1 leading-tight">
            {{ getChapterDisplayTitle(item.chapter!) }}
          </span>

          <span class="flex items-center gap-1 shrink-0">
            <span
              v-if="item.chapter!.analyzed"
              class="text-green-500 dark:text-green-400 text-[10px]"
              title="已分析"
            >
              ✓
            </span>
            <span
              :class="[
                'text-[10px] font-mono',
                isCurrentChapter(item.chapter!) ? 'text-primary-500' : 'text-fabric-thread/40'
              ]"
            >
              {{ formatCharCount(item.chapter!.char_count) }}
            </span>
          </span>
        </button>
      </template>
    </VirtualList>

    <div
      v-if="chapters.length === 0"
      class="text-center text-fabric-thread/50 text-xs py-6"
    >
      暂无章节
    </div>
  </div>
</template>
