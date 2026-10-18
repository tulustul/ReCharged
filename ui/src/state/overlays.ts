// Dialogs and toasts (pure stores + helpers). Hosts in App render them above every screen.
import { createStore, hmrSingleton } from "./store.ts";

export type DialogTone = "primary" | "secondary" | "danger";

export type DialogAction = { label: string; tone?: DialogTone; run?: () => void };

export type DialogSpec = {
  id: number;
  title: string;
  text?: string;
  /** Technical detail, small. */
  detail?: string;
  icon?: "warning" | "controller";
  actions: DialogAction[];
  /** Index of the action Back triggers (none: Back does nothing — a choice is required). */
  cancel?: number;
  /** Initially focused action. */
  focus?: number;
};

export type ToastTone = "info" | "success" | "error" | "unlock";

export type ToastSpec = { id: number; tone: ToastTone; title: string; text?: string };

/** Toasts shown at once; older ones drop off. */
export const MAX_TOASTS = 4;

const state = hmrSingleton("__rc_overlays", () => ({
  dialogs: createStore<DialogSpec[]>([]),
  toasts: createStore<ToastSpec[]>([]),
  nextId: 1,
}));

export const dialogs = state.dialogs;
export const toasts = state.toasts;

export function openDialog(spec: Omit<DialogSpec, "id">): number {
  const id = state.nextId++;
  dialogs.update((list) => [...list, { ...spec, id }]);
  return id;
}

export function closeDialog(id: number) {
  dialogs.update((list) => list.filter((d) => d.id !== id));
}

export function pushToast(toast: Omit<ToastSpec, "id">): number {
  const id = state.nextId++;
  toasts.update((list) => [...list, { ...toast, id }].slice(-MAX_TOASTS));
  return id;
}

export function dismissToast(id: number) {
  toasts.update((list) => list.filter((t) => t.id !== id));
}
