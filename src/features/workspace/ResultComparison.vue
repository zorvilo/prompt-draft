<script setup lang="ts">
import type { Change } from "diff";

defineProps<{ source: string; result: string; changes?: Change[] }>();
</script>

<template>
  <section
    class="comparison-view result-content"
    tabindex="0"
    aria-label="优化稿对比"
  >
    <header class="comparison-heading">
      <div><strong>修订对照</strong><span>原稿与优化稿的文字变化</span></div>
      <div class="comparison-legend" aria-label="修订标记说明">
        <span class="comparison-removed">删除</span>
        <span class="comparison-added">新增</span>
      </div>
    </header>
    <pre
      v-if="changes"
      class="comparison"
    ><template v-for="(chunk, index) in changes" :key="index"><del v-if="chunk.removed">{{ chunk.value }}</del><ins v-else-if="chunk.added">{{ chunk.value }}</ins><span v-else>{{ chunk.value }}</span></template></pre>
    <div v-else class="comparison-fallback">
      <section>
        <h3>原稿</h3>
        <pre class="comparison"><del>{{ source }}</del></pre>
      </section>
      <section>
        <h3>优化稿</h3>
        <pre class="comparison"><ins>{{ result }}</ins></pre>
      </section>
    </div>
  </section>
</template>

<style scoped>
.comparison-view {
  background: var(--paper);
  padding: 20px;
}
.comparison-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 0 14px;
  margin-bottom: 20px;
  border-bottom: 1px solid var(--line);
}
.comparison-heading > div:first-child {
  display: grid;
  gap: 3px;
}
.comparison-heading strong {
  font-size: 15px;
}
.comparison-heading span {
  font-size: 12px;
}
.comparison-heading > div:first-child span {
  color: var(--muted);
}
.comparison-legend {
  display: flex;
  gap: 8px;
}
.comparison-legend span {
  padding: 3px 9px;
  border-radius: 4px;
  font-weight: 600;
}
.comparison-removed {
  color: var(--removed-ink);
  background: var(--before);
  text-decoration: line-through;
}
.comparison-added {
  color: var(--added-ink);
  background: var(--after);
  text-decoration: underline;
  text-underline-offset: 3px;
}
.comparison {
  font-size: 16px;
  line-height: 2.1;
}
.comparison-fallback {
  display: grid;
  gap: 24px;
}
.comparison-fallback h3 {
  margin: 0 0 10px;
  font-size: 13px;
  color: var(--muted);
}
@media (max-width: 660px) {
  .comparison-view {
    padding: 14px;
  }
}
</style>
