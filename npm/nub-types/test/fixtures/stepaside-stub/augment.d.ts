// Models a consumer who, on top of `lib: ["dom"]`, AUGMENTS the global `Worker`
// with their own member (a real, common pattern). Because @nubjs/types steps aside
// when lib.dom is present, its `Worker`/`var Worker` do not occupy the slot, so the
// user's augmentation merges cleanly onto lib.dom's `Worker` with NO TS2403 from
// @nubjs/types. This is the second step-aside scenario: @nubjs/types coexisting with
// BOTH lib.dom and a user-side global augmentation of the same name.
declare global {
  interface Worker {
    nubTag?: string;
  }
}
export {};
