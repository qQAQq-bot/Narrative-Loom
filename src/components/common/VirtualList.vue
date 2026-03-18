<script setup lang="ts" generic="T">
/**
 * VirtualList - 虚拟滚动列表组件 (P4-052)
 *
 * 只渲染可视区域内的元素，适用于长列表场景
 * 支持固定高度和动态高度两种模式
 */
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'

interface Props {
  items: T[]
  itemHeight?: number // 固定高度模式
  estimatedItemHeight?: number // 动态高度模式的估算高度
  buffer?: number // 缓冲区大小（上下各多渲染几个元素）
  keyField?: string // 用于 key 的字段名
}

const props = withDefaults(defineProps<Props>(), {
  itemHeight: 0,
  estimatedItemHeight: 50,
  buffer: 3,
  keyField: 'id'
})

const emit = defineEmits<{
  (e: 'scroll', scrollTop: number): void
  (e: 'scrollEnd'): void
}>()

// 容器引用
const containerRef = ref<HTMLElement | null>(null)
const contentRef = ref<HTMLElement | null>(null)

// 滚动状态
const scrollTop = ref(0)
const containerHeight = ref(0)

// 动态高度缓存
const itemHeights = ref<Map<number, number>>(new Map())

// 计算每个元素的实际高度
function getItemHeight(index: number): number {
  if (props.itemHeight > 0) {
    return props.itemHeight
  }
  return itemHeights.value.get(index) || props.estimatedItemHeight
}

// 计算总高度
const totalHeight = computed(() => {
  if (props.itemHeight > 0) {
    return props.items.length * props.itemHeight
  }

  let height = 0
  for (let i = 0; i < props.items.length; i++) {
    height += getItemHeight(i)
  }
  return height
})

// 计算可见范围
const visibleRange = computed(() => {
  if (props.items.length === 0) {
    return { start: 0, end: 0 }
  }

  // If container height is not yet measured, show initial items
  const effectiveContainerHeight = containerHeight.value || 500

  let start = 0
  let accumulatedHeight = 0

  // 找到起始索引
  if (props.itemHeight > 0) {
    start = Math.floor(scrollTop.value / props.itemHeight)
  } else {
    for (let i = 0; i < props.items.length; i++) {
      const height = getItemHeight(i)
      if (accumulatedHeight + height > scrollTop.value) {
        start = i
        break
      }
      accumulatedHeight += height
    }
  }

  // 计算结束索引
  let end = start
  let visibleHeight = 0
  const targetHeight = effectiveContainerHeight

  if (props.itemHeight > 0) {
    end = Math.ceil((scrollTop.value + targetHeight) / props.itemHeight)
  } else {
    for (let i = start; i < props.items.length; i++) {
      visibleHeight += getItemHeight(i)
      end = i + 1
      if (visibleHeight >= targetHeight) {
        break
      }
    }
  }

  // 应用缓冲区
  start = Math.max(0, start - props.buffer)
  end = Math.min(props.items.length, end + props.buffer)

  return { start, end }
})

// 可见元素
const visibleItems = computed(() => {
  const { start, end } = visibleRange.value
  return props.items.slice(start, end).map((item, index) => ({
    item,
    index: start + index
  }))
})

// 计算偏移量
const offsetY = computed(() => {
  const { start } = visibleRange.value

  if (props.itemHeight > 0) {
    return start * props.itemHeight
  }

  let offset = 0
  for (let i = 0; i < start; i++) {
    offset += getItemHeight(i)
  }
  return offset
})

// 获取元素的 key
function getItemKey(item: T, index: number): string | number {
  if (props.keyField && typeof item === 'object' && item !== null) {
    return (item as Record<string, unknown>)[props.keyField] as string | number
  }
  return index
}

// 滚动处理
let scrollTimeout: ReturnType<typeof setTimeout> | null = null

function handleScroll(event: Event) {
  const target = event.target as HTMLElement
  scrollTop.value = target.scrollTop
  emit('scroll', scrollTop.value)

  // 检测滚动结束
  if (scrollTimeout) {
    clearTimeout(scrollTimeout)
  }
  scrollTimeout = setTimeout(() => {
    emit('scrollEnd')
  }, 150)
}

// 更新元素高度（用于动态高度模式）
function updateItemHeight(index: number, height: number) {
  if (props.itemHeight === 0 && height > 0) {
    itemHeights.value.set(index, height)
  }
}

// 滚动到指定索引
function scrollToIndex(index: number, behavior: ScrollBehavior = 'smooth') {
  if (!containerRef.value) return

  let targetOffset = 0
  if (props.itemHeight > 0) {
    targetOffset = index * props.itemHeight
  } else {
    for (let i = 0; i < index; i++) {
      targetOffset += getItemHeight(i)
    }
  }

  containerRef.value.scrollTo({
    top: targetOffset,
    behavior
  })
}

// 滚动到顶部
function scrollToTop(behavior: ScrollBehavior = 'smooth') {
  scrollToIndex(0, behavior)
}

// 滚动到底部
function scrollToBottom(behavior: ScrollBehavior = 'smooth') {
  if (!containerRef.value) return
  containerRef.value.scrollTo({
    top: totalHeight.value,
    behavior
  })
}

// 初始化
function initContainer() {
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight
  }
}

// ResizeObserver
let resizeObserver: ResizeObserver | null = null

onMounted(() => {
  initContainer()

  if (containerRef.value) {
    resizeObserver = new ResizeObserver(() => {
      initContainer()
    })
    resizeObserver.observe(containerRef.value)
  }
})

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
  }
  if (scrollTimeout) {
    clearTimeout(scrollTimeout)
  }
})

// 暴露方法
defineExpose({
  scrollToIndex,
  scrollToTop,
  scrollToBottom,
  updateItemHeight
})
</script>

<template>
  <div
    ref="containerRef"
    class="virtual-list-container"
    @scroll="handleScroll"
  >
    <div
      ref="contentRef"
      class="virtual-list-content"
      :style="{ height: `${totalHeight}px` }"
    >
      <div
        class="virtual-list-items"
        :style="{ transform: `translateY(${offsetY}px)` }"
      >
        <div
          v-for="{ item, index } in visibleItems"
          :key="getItemKey(item, index)"
          class="virtual-list-item"
          :data-index="index"
        >
          <slot :item="item" :index="index" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.virtual-list-container {
  overflow-y: auto;
  height: 100%;
  position: relative;
}

.virtual-list-content {
  position: relative;
}

.virtual-list-items {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
}

.virtual-list-item {
  width: 100%;
}
</style>
