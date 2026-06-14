// STEP-ASIDE fixture (the key new test) — a consumer that ALSO has `Worker` globally
// via `lib: ["dom"]`. Before the step-aside idiom, @nubjs/types' own `interface
// Worker` + `declare var Worker` collided with lib.dom's (TS2403 / TS2430). With the
// idiom, @nubjs/types detects lib.dom (via the global `onabort`), steps aside, and
// adopts the DOM `Worker` — so the two coexist.
// Expected: tsc --noEmit exits 0 (NO TS2403/TS2430). Worker resolves to lib.dom's.

// lib.dom's Worker — must work, no collision from @nubjs/types redeclaring it.
const worker = new Worker(new URL("./worker.js", import.meta.url));
worker.postMessage({ ping: true });
worker.terminate();

// @nubjs/types surfaces that DON'T overlap lib.dom must STILL resolve alongside dom.
reportError(new Error("boom"));
const instant: Temporal.Instant = Temporal.Now.instant();
console.log(instant.toString());
