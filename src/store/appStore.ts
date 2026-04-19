import { useSyncExternalStore } from "react";
import type { AiStatus, CharacterId } from "../lib/sprites";

export interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface AiError {
  text: string;
  /** Unix ms at which Character.tsx should clear this bubble. */
  expiresAt: number;
}

export type AgentKind = "claude" | "codex" | "gemini";

export interface AgentInfo {
  kind: AgentKind;
  path: string;
  displayName: string;
}

export interface AgentPicker {
  agents: AgentInfo[];
  /** Unix ms at which the picker auto-dismisses if user doesn't pick. */
  expiresAt: number;
}

interface AppState {
  aiStatus: Record<CharacterId, AiStatus>;
  bounds: Record<CharacterId, Bounds | null>;
  errors: Record<CharacterId, AiError | null>;
  pickers: Record<CharacterId, AgentPicker | null>;
}

const state: AppState = {
  aiStatus: { bruce: "idle", jazz: "idle" },
  bounds: { bruce: null, jazz: null },
  errors: { bruce: null, jazz: null },
  pickers: { bruce: null, jazz: null },
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
  /**
   * Flash a red error bubble above the character for `durationMs`.
   * Called when a Tauri command (e.g. spawn_claude) returns an error
   * that the user should actually see — as opposed to benign dedupes.
   */
  setError(id: CharacterId, text: string, durationMs = 5000) {
    state.errors[id] = { text, expiresAt: Date.now() + durationMs };
    emit();
  },
  clearError(id: CharacterId) {
    if (state.errors[id] !== null) {
      state.errors[id] = null;
      emit();
    }
  },
  /** Show a picker bubble with clickable agent options on this character. */
  setPicker(id: CharacterId, agents: AgentInfo[], durationMs = 10000) {
    state.pickers[id] = { agents, expiresAt: Date.now() + durationMs };
    emit();
  },
  clearPicker(id: CharacterId) {
    if (state.pickers[id] !== null) {
      state.pickers[id] = null;
      emit();
    }
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
