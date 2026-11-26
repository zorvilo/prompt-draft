<template>
  <span
    ref="viewport"
    class="hover-scroll-text"
    :class="{ scrolling }"
    @mouseenter="enter"
    @mouseleave="leave"
    ><span ref="content" class="hover-scroll-content">{{ text }}</span></span
  >
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
const props = defineProps<{ text: string }>();
const viewport = ref<HTMLElement>();
const content = ref<HTMLElement>();
const scrolling = ref(false);
let hovered = false;
let animation: Animation | undefined;
let observer: ResizeObserver;
function reset() {
  animation?.cancel();
  animation = undefined;
  scrolling.value = false;
}
function start() {
  reset();
  if (!hovered || !viewport.value || !content.value) return;
  const distance = Math.ceil(
    content.value.getBoundingClientRect().width - viewport.value.clientWidth,
  );
  if (distance <= 0) return;
  scrolling.value = true;
  animation = content.value.animate(
    [
      { transform: "translateX(0)" },
      { transform: `translateX(-${distance}px)` },
    ],
    {
      delay: 400,
      duration: (distance / 35) * 1000,
      easing: "linear",
      fill: "forwards",
    },
  );
}
function enter() {
  hovered = true;
  start();
}
function leave() {
  hovered = false;
  reset();
}
onMounted(() => {
  observer = new ResizeObserver(start);
  if (viewport.value) observer.observe(viewport.value);
});
watch(
  () => props.text,
  async () => {
    await nextTick();
    start();
  },
);
onUnmounted(() => {
  observer?.disconnect();
  reset();
});
</script>

<style scoped>
.hover-scroll-text {
  display: block;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.hover-scroll-text.scrolling {
  text-overflow: clip;
}
.hover-scroll-text .hover-scroll-content {
  display: inline;
  max-width: none;
  overflow: visible;
  vertical-align: top;
}
.hover-scroll-text.scrolling .hover-scroll-content {
  display: inline-block;
  width: max-content;
}
</style>
