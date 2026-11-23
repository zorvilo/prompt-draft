<script setup lang="ts">
import { computed } from "vue";
import { diffChars } from "diff";

const props = defineProps<{ source: string; result: string }>();
const changes = computed(() =>
  diffChars(props.source, props.result, { timeout: 150 }),
);
</script>

<template>
  <section
    class="comparison-view result-content"
    tabindex="0"
    aria-label="优化稿对比"
  >
    <header class="comparison-heading">
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
  margin-bottom: 12px;
}
.comparison-legend {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  font-size: 12px;
}
.comparison-legend span {
  padding: 2px 6px;
  border-radius: 4px;
}
.comparison-removed,
.comparison del {
  color: var(--removed-ink);
  background: color-mix(in srgb, var(--before) 60%, var(--paper));
  text-decoration: line-through;
  text-decoration-thickness: 1px;
}
.comparison-added,
.comparison ins {
  color: var(--added-ink);
  background: color-mix(in srgb, var(--after) 60%, var(--paper));
  text-decoration: none;
  font-weight: inherit;
}
.comparison {
  font-size: 15px;
  line-height: 1.85;
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
