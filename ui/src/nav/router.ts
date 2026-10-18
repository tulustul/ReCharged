// Screen stack (pure): Back pops one level; the bottom screen never pops.

export const top = <R>(stack: readonly R[]): R => stack[stack.length - 1];

export const push = <R>(stack: readonly R[], route: R): R[] => [...stack, route];

export const pop = <R>(stack: readonly R[]): R[] => (stack.length > 1 ? stack.slice(0, -1) : [...stack]);

export const replace = <R>(stack: readonly R[], route: R): R[] => [...stack.slice(0, -1), route];

/** Pops back to the last route matching `pred` (kept), or to the bottom when none matches. */
export function popTo<R>(stack: readonly R[], pred: (route: R) => boolean): R[] {
  for (let i = stack.length - 1; i >= 0; i--) if (pred(stack[i])) return stack.slice(0, i + 1);
  return stack.slice(0, 1);
}
