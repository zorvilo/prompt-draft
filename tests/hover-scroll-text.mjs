import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parse } from "@vue/compiler-sfc";
import ts from "typescript";
const source = parse(
  readFileSync(
    new URL("../src/components/HoverScrollText.vue", import.meta.url),
    "utf8",
  ),
).descriptor.scriptSetup.content.replace(/^import .*?from "vue";$/m, "");
const code = ts.transpile(source, {
  target: ts.ScriptTarget.ES2022,
  module: ts.ModuleKind.None,
});
let mounted, unmounted, resized;
const create = new Function(
  "ref",
  "defineProps",
  "onMounted",
  "onUnmounted",
  "watch",
  "nextTick",
  "ResizeObserver",
  code + "\nreturn {viewport,content,scrolling,enter,leave};",
);
let cancelled = 0;
const animations = [];
const instance = create(
  () => ({ value: undefined }),
  () => ({ text: "long title" }),
  (fn) => {
    mounted = fn;
  },
  (fn) => {
    unmounted = fn;
  },
  () => {},
  () => Promise.resolve(),
  class {
    constructor(fn) {
      resized = fn;
    }
    observe() {}
    disconnect() {}
  },
);
instance.viewport.value = { clientWidth: 100 };
instance.content.value = {
  getBoundingClientRect: () => ({ width: 240 }),
  animate: (frames, options) => {
    animations.push({ frames, options });
    return {
      cancel: () => {
        cancelled++;
      },
    };
  },
};
mounted();
resized();
assert.equal(animations.length, 0, "never animate without hover");
instance.enter();
assert.equal(instance.scrolling.value, true);
assert.equal(animations[0].frames[0].transform, "translateX(0)");
assert.equal(animations[0].frames[1].transform, "translateX(-140px)");
assert.equal(animations[0].options.duration, 4000);
assert.equal(animations[0].options.fill, "forwards");
instance.leave();
assert.equal(cancelled, 1);
assert.equal(instance.scrolling.value, false);
instance.viewport.value.clientWidth = 300;
instance.enter();
assert.equal(animations.length, 1, "short titles stay still");
instance.viewport.value.clientWidth = 80;
resized();
assert.equal(animations[1].frames[1].transform, "translateX(-160px)");
unmounted();
assert.equal(cancelled, 2);
console.log("Hover title scroll checks passed.");
