import assert from "node:assert/strict";
import { visibleMessageIds } from "../src/utils/workspace/messageVisibility.ts";
const messages = [{ id: "one", top: -60, bottom: 0 }, { id: "two", top: 20, bottom: 60 }, { id: "three", top: 180, bottom: 230 }, { id: "four", top: 300, bottom: 340 }];
assert.deepEqual(visibleMessageIds({ top: 0, bottom: 200 }, messages), ["two", "three"]);
assert.deepEqual(visibleMessageIds({ top: 100, bottom: 300 }, messages), ["three"]);
assert.deepEqual(visibleMessageIds({ top: 70, bottom: 170 }, messages), []);
assert.deepEqual(visibleMessageIds({ top: 0, bottom: 500 }, messages), ["two", "three", "four"]);
assert.deepEqual(visibleMessageIds({ top: 0, bottom: 200 }, []), []);
console.log("visible prompts highlight together, including partial visibility and no-visible-prompt cases");
