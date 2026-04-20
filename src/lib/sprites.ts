export type CharacterId = "bruce" | "jazz";

export type AiStatus = "idle" | "busy" | "completed";

export interface CharacterConfig {
  sheetSrc: string;
}

export const CHARACTERS: Record<CharacterId, CharacterConfig> = {
  bruce: { sheetSrc: "/sprites/walk-bruce.png" },
  jazz: { sheetSrc: "/sprites/walk-jazz.png" },
};

// Display size in CSS pixels. The sprite frame (after cropping away empty
// vertical padding in convert-sprites.mjs) has aspect 1080:1330, so at
// DISPLAY_HEIGHT=160 the width is ~130. The character's feet sit exactly
// at the bottom of the frame, so translating the sprite to
// `top = trackBottom - DISPLAY_HEIGHT` puts the feet on the taskbar edge.
export const DISPLAY_HEIGHT = 160;
export const DISPLAY_WIDTH = Math.round(DISPLAY_HEIGHT * (1080 / 1330));

// Sprite-sheet grid (must match scripts/convert-sprites.mjs)
export const SHEET_COLS = 15;
export const SHEET_ROWS = 10;
export const SHEET_FRAMES = SHEET_COLS * SHEET_ROWS;
export const SHEET_FPS = 15;

export const WALK_TIMING = {
  videoDuration: 10.0,
  accelStart: 3.0,
  fullSpeedStart: 3.75,
  decelStart: 7.5,
  walkStop: 8.25,
};

export const WALK_AMOUNT_RANGE: [number, number] = [0.25, 0.5];
export const REFERENCE_WIDTH = 500;
export const PAUSE_RANGE_SEC: [number, number] = [5, 12];

export function movementPosition(videoTime: number): number {
  const { accelStart, fullSpeedStart, decelStart, walkStop } = WALK_TIMING;
  const dIn = fullSpeedStart - accelStart;
  const dLin = decelStart - fullSpeedStart;
  const dOut = walkStop - decelStart;
  const v = 1.0 / (dIn / 2.0 + dLin + dOut / 2.0);

  if (videoTime <= accelStart) return 0.0;
  if (videoTime <= fullSpeedStart) {
    const t = videoTime - accelStart;
    return (v * t * t) / (2.0 * dIn);
  }
  if (videoTime <= decelStart) {
    const easeIn = (v * dIn) / 2.0;
    const t = videoTime - fullSpeedStart;
    return easeIn + v * t;
  }
  if (videoTime <= walkStop) {
    const easeIn = (v * dIn) / 2.0;
    const linear = v * dLin;
    const t = videoTime - decelStart;
    return easeIn + linear + v * (t - (t * t) / (2.0 * dOut));
  }
  return 1.0;
}

export const THINKING_PHRASES = [
  "hmm...", "thinking...", "one sec...", "ok hold on",
  "let me check", "working on it", "almost...", "bear with me",
  "on it!", "gimme a sec", "brb", "processing...",
  "hang tight", "just a moment", "figuring it out",
  "crunching...", "reading...", "looking...",
  "consulting the oracle...", "running the numbers...",
  "checking my notes...", "doing the math", "reasoning...",
  "stirring the pot", "cooking...", "thinking very hard",
  "let me think...", "noodle-ing on this", "plotting...",
  "wiring it up", "untangling...", "tracing it back",
  "chewing on this", "in the zone", "deep breath",
  "scribbling...", "sketching...", "pondering",
  "weighing options", "double-checking", "triangulating",
  "lemme see", "one moment", "putting it together",
];

export const COMPLETION_PHRASES = [
  "done!", "all set!", "ready!", "here you go", "got it!",
  "finished!", "ta-da!", "voila!", "boom!", "there ya go!",
  "nailed it", "sorted", "that's the one", "ship it",
];

export function randomPhrase(list: string[], exclude?: string): string {
  if (list.length === 0) return "";
  let pick = list[Math.floor(Math.random() * list.length)];
  if (exclude && list.length > 1) {
    while (pick === exclude) {
      pick = list[Math.floor(Math.random() * list.length)];
    }
  }
  return pick;
}

export const COMPLETION_SOUNDS = [
  "/sounds/ping-aa.mp3", "/sounds/ping-bb.mp3", "/sounds/ping-cc.mp3",
  "/sounds/ping-dd.mp3", "/sounds/ping-ee.mp3", "/sounds/ping-ff.mp3",
  "/sounds/ping-gg.mp3", "/sounds/ping-hh.mp3",
];
