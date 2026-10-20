// Pure helpers of the editor's asset library (search, matching assets).
import type { EditorAsset, LibraryGroup } from "../bevy";
import { createStore, hmrSingleton } from "../state/store.ts";

/** The Assets tab's search text (kept across tab switches; `editor search <text>` scripts it). */
export const libraryQuery = hmrSingleton("__rc_libraryQuery", () => createStore(""));

/** Two library assets are the same asset. */
export function sameAsset(a: EditorAsset | null | undefined, b: EditorAsset | null | undefined): boolean {
  return a != null && b != null && JSON.stringify(a) === JSON.stringify(b);
}

/** The groups whose entries match every word of `query` (and are of `kind`, if given); empty groups go. */
export function filterGroups(groups: LibraryGroup[], query: string, kind?: EditorAsset["kind"]): LibraryGroup[] {
  const words = query.toLowerCase().split(/\s+/).filter((w) => w !== "");
  return groups
    .map((g) => ({
      ...g,
      entries: g.entries.filter((e) => (kind === undefined || e.asset.kind === kind) && words.every((w) => e.name.toLowerCase().includes(w))),
    }))
    .filter((g) => g.entries.length > 0);
}

/** The shown name of `asset` in the library (its id when it is not there). */
export function assetName(groups: LibraryGroup[], asset: EditorAsset): string {
  const entry = groups.flatMap((g) => g.entries).find((e) => sameAsset(e.asset, asset));
  if (entry) return entry.name;
  return asset.kind === "prop" ? asset.id : asset.kind === "decor" ? asset.model : asset.kind;
}
