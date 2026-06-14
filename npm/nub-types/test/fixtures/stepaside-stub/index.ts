// STEP-ASIDE (lib.dom + user augmentation) — the consumer has lib.dom AND augments the
// global `Worker` (see augment.d.ts). @nubjs/types steps aside for lib.dom, so neither
// its `Worker` interface nor its `var Worker` collides (no TS2403), and the user's
// `nubTag` augmentation lands on lib.dom's `Worker`.
// Expected: tsc --noEmit exits 0. `Worker` is lib.dom's, plus the user's `nubTag`.

const worker = new Worker(new URL("./worker.js", import.meta.url));
worker.postMessage({ ping: true });
worker.nubTag = "from-augmentation"; // user's augmentation member resolves
worker.terminate();

// Non-overlapping @nubjs/types surfaces still resolve alongside dom + augmentation.
reportError(new Error("boom"));
const instant: Temporal.Instant = Temporal.Now.instant();
console.log(instant.toString());
