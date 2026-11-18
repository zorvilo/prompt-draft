<script setup lang="ts">
import type { Change } from "diff";

defineProps<{ source: string; result: string; changes?: Change[] }>();
</script>

<template>
  <pre
    class="comparison result-content"
    tabindex="0"
    aria-label="优化稿对比"
  ><template v-if="changes"><span class="diff-key"><del>删除</del> <ins>新增</ins></span><template v-for="(chunk, index) in changes" :key="index"><del v-if="chunk.removed">{{ chunk.value }}</del><ins v-else-if="chunk.added">{{ chunk.value }}</ins><span v-else>{{ chunk.value }}</span></template></template><template v-else><span class="before-label">原稿</span><del>{{ source }}</del><span class="after-label">优化稿</span><ins>{{ result }}</ins></template></pre>
</template>
