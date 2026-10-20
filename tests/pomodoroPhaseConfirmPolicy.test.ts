import assert from "node:assert/strict";
import { isDeliberateMouseClick } from "../src/features/tools/services/pomodoroPhaseConfirmPolicy.ts";

async function runTest(name: string, fn: () => Promise<void> | void) {
  try {
    await fn();
    console.log(`ok - ${name}`);
  } catch (error) {
    console.error(`not ok - ${name}`);
    throw error;
  }
}

await runTest("a real mouse press counts as a confirmation", () => {
  // The DOM reports the click count for pointer input, so a single press is 1 and a
  // double is 2; both are deliberate.
  assert.equal(isDeliberateMouseClick(1), true);
  assert.equal(isDeliberateMouseClick(2), true);
});

await runTest("a keyboard activation does not count", () => {
  // Enter or Space on a focused button fires a click whose `detail` is 0. A finished
  // pomodoro phase is held by its alert, which cannot be cleared any other way, so
  // letting this through would start the next phase on a stray keypress.
  assert.equal(isDeliberateMouseClick(0), false);
});

await runTest("an implausible click count is still treated as a press", () => {
  // Only zero is special; anything positive came from pointer input.
  assert.equal(isDeliberateMouseClick(255), true);
});
