import { useSyncExternalStore } from "react";
import type { AiStatus, CharacterId } from "../lib/sprites";

export interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface AppState {
  aiStatus: Record<CharacterId, AiStatus>;
  bounds: Record<CharacterId, Bounds | null>;
}

const state: AppState = {
  aiStatus: { bruce: "idle", jazz: "idle" },
  bounds: { bruce: null, jazz: null },
};

const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

export const appStore = {
  get(): AppState {
    return state;
  },
  setAiStatus(id: CharacterId, s: AiStatus) {
    state.aiStatus[id] = s;
    emit();
  },
  setBounds(id: CharacterId, b: Bounds) {
    state.bounds[id] = b;
    emit();
  },
  subscribe(l: () => void) {
    listeners.add(l);
    return () => listeners.delete(l);
  },
};

export function useAppStore<T>(selector: (s: AppState) => T): T {
  return useSyncExternalStore(
    (l) => appStore.subscribe(l),
    () => selector(appStore.get()),
    () => selector(appStore.get()),
  );
}
