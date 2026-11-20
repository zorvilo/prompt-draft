import assert from "node:assert/strict";
import { routeTemplateWheel } from "../src/features/workspace/templateWheel.ts";
let focused = false;
let moved;
let prevented;
let stopped;
const page = { clientHeight: 500, scrollBy: (value) => { moved = value; } };
const field = {
  closest: (selector) => {
    // The template page scrolls inside utility-scroll, not main-surface.
    assert.equal(selector, ".utility-scroll, .main-surface");
    return page;
  },
  contains: () => focused,
  ownerDocument: { activeElement: {} },
};
function wheel(overrides = {}) {
  moved = undefined;
  prevented = stopped = false;
  routeTemplateWheel({
    currentTarget: field, cancelable: true, ctrlKey: false,
    defaultPrevented: false, deltaMode: 0, deltaY: 40, deltaX: 2,
    preventDefault() { prevented = true; },
    stopPropagation() { stopped = true; },
    ...overrides,
  });
}
wheel();
assert.deepEqual(moved, { top: 40, left: 2 });
assert.ok(prevented && stopped);
wheel({ deltaMode: 1 });
assert.equal(moved.top, 640);
wheel({ deltaMode: 2 });
assert.equal(moved.top, 20000);
focused = true;
wheel();
assert.equal(moved, undefined);
assert.equal(prevented, false);
focused = false;
for (const overrides of [{ ctrlKey: true }, { defaultPrevented: true }, { cancelable: false }]) {
  wheel(overrides);
  assert.equal(prevented, false);
}
console.log("template wheel passed");
