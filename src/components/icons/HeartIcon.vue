<script setup lang="ts">
// 侧栏图标集。
//
// Element Plus 未提供心形图标（仅有 Star / StarFilled）。
// 为避免引入额外图标库，此处以 SVG 绘制心形，描边与尺寸对齐 Element Plus 图标风格。

withDefaults(defineProps<{ size?: number }>(), { size: 17 })
</script>

<template>
  <!-- 爱心：点赞 -->
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    class="ico"
  >
    <path
      d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1-1.1a5.5 5.5 0 0 0-7.8 7.8l1 1.1L12 21.2l7.8-7.7 1-1.1a5.5 5.5 0 0 0 0-7.8z"
    />
  </svg>
</template>

<style scoped>
.ico {
  display: block;
}
</style>
