// NEGATIVE CONTROL — proves the global-script invariant. The runner generates
// `nub-env-as-module.d.ts` = the current index.d.ts WITH `export {}` appended, which
// turns it into a module. Once it is a module, the wildcard `declare module "*.yaml"`
// and the bare globals (reportError) are NO LONGER project-visible — so this fixture
// MUST FAIL to typecheck. A pass here means the file silently became a module and the
// data-import wildcards / globals broke.
// Expected: tsc --noEmit exits NON-ZERO (TS2307 for the .yaml import, TS2304 for reportError).

import cfg from "./config.yaml";
console.log(cfg);
reportError(new Error("boom"));
