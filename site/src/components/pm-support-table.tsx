import type { ReactNode } from 'react';

// At-a-glance support summary: one row per INFERRED package manager, with a
// flat set of code chips for the config files / package.json fields / env-var
// prefixes it reads. A green check = honored, a red X = a deliberate gap.
// Chips are grounded in each PM's detailed CompatTable on its own install page.

interface PmItem {
  /** A config file, package.json field, or env-var glob — rendered as a code chip. */
  code: string;
  ok: boolean;
  /** Amber glyph — partial support (e.g. read-only). Overrides `ok` for the icon. */
  partial?: boolean;
}

interface PmRow {
  pm: ReactNode;
  /** Link the PM name (left column) to its full per-PM write-up. */
  href?: string;
  /** Small qualifier after the PM name, e.g. "read-only". */
  qualifier?: string;
  /** Accent the row — used for nub's own identity row at the bottom. */
  highlight?: boolean;
  items: PmItem[];
}

function CheckIcon() {
  return (
    <svg
      aria-hidden
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="3.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="size-3 shrink-0 text-emerald-600 dark:text-emerald-400"
    >
      <path d="M20 6 9 17l-5-5" />
    </svg>
  );
}

function PartialIcon() {
  return (
    <svg
      aria-hidden
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="3.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="size-3 shrink-0 text-amber-500 dark:text-amber-400"
    >
      <path d="M20 6 9 17l-5-5" />
    </svg>
  );
}

function CrossIcon() {
  return (
    <svg
      aria-hidden
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="3.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="size-3 shrink-0 text-red-500 dark:text-red-400"
    >
      <path d="M18 6 6 18" />
      <path d="m6 6 12 12" />
    </svg>
  );
}

// Sort rank so chips render supported → partial → unsupported (greens first).
const chipRank = (it: PmItem) => (it.partial ? 1 : it.ok ? 0 : 2);

function Chip({ code, ok, partial }: PmItem) {
  return (
    <span className="inline-flex items-center gap-1.5 rounded-md border border-fd-border bg-fd-muted/40 px-2 py-1 font-mono text-xs">
      {partial ? <PartialIcon /> : ok ? <CheckIcon /> : <CrossIcon />}
      <span className={partial || ok ? 'text-fd-foreground' : 'text-fd-muted-foreground'}>
        {code}
      </span>
    </span>
  );
}

export function PmSupport({ rows }: { rows: PmRow[] }) {
  return (
    <div className="my-6 overflow-hidden rounded-lg border border-fd-border [&_table]:my-0">
      <table className="w-full border-collapse text-left text-sm">
        <thead>
          <tr className="border-b border-fd-border bg-fd-muted/40">
            <th className="w-px whitespace-nowrap px-4 py-2.5 font-medium text-fd-muted-foreground">
              Package manager
            </th>
            <th className="px-4 py-2.5 font-medium text-fd-muted-foreground">Config it reads</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr
              key={i}
              className={`border-b border-fd-border/60 align-top last:border-0 ${
                r.highlight ? 'bg-pink/[0.04]' : ''
              }`}
            >
              <td
                className={`whitespace-nowrap px-4 py-3 font-mono font-medium ${
                  r.highlight ? 'text-pink' : 'text-fd-foreground'
                }`}
              >
                {r.href ? (
                  <a href={r.href} className="underline decoration-fd-border underline-offset-4 hover:decoration-current">
                    {r.pm}
                  </a>
                ) : (
                  r.pm
                )}
                {r.qualifier ? (
                  <span className="ml-1.5 font-sans text-xs font-normal text-fd-muted-foreground">
                    {r.qualifier}
                  </span>
                ) : null}
              </td>
              <td className="px-4 py-3">
                <span className="flex flex-wrap gap-1.5">
                  {[...r.items]
                    .sort((a, b) => chipRank(a) - chipRank(b))
                    .map((it, j) => (
                      <Chip key={j} {...it} />
                    ))}
                </span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
