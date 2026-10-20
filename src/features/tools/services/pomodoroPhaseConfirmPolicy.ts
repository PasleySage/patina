/**
 * Whether a click event came from a deliberate mouse press.
 *
 * The DOM reports a `click` event's `detail` as the click count for pointer input and 0
 * for anything synthesized by the keyboard — Enter or Space activating a focused button,
 * or an accessibility tool dispatching a synthetic click. Both arrive as ordinary click
 * events with no other distinguishing field, so `detail` is the only signal available.
 *
 * A finished pomodoro phase needs this because its alert is the gate that holds the run
 * until the user decides to continue, and that alert cannot be cleared any other way.
 * A keypress that happened to land on the focused button would otherwise start the next
 * phase, which is exactly the accidental advance the gate exists to prevent.
 */
export function isDeliberateMouseClick(detail: number): boolean {
  return detail > 0;
}
