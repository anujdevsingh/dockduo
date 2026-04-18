#!/usr/bin/env node
// Convert MConverter.eu PNG sequences (RGB, black background) into transparent
// sprite-sheet PNGs for Chromium/WebView2 playback. Also copies sounds and
// bubble icon from the upstream Mac repo.
//
// The upstream .mov files are HEVC-with-alpha (Apple-only). FFmpeg on Windows
// cannot decode the alpha channel. Workaround: we ran the .mov files through
// MConverter.eu which gave us PNG sequences on a pure-black background. We
// now strip the black via colorkey and tile into the sprite sheet.

import { execSync } from "node:child_process";
import { existsSync, mkdirSync, copyFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";

const ROOT = resolve(new URL("..", import.meta.url).pathname.replace(/^\//, ""));
const UPSTREAM = join(ROOT, "lil-agents-main", "LilAgents");
const FRAMES_BRUCE = join(ROOT, "_MConverter.eu_walk-bruce-01");
const FRAMES_JAZZ = join(ROOT, "_MConverter.eu_walk-jazz-01");
const OUT_SPRITES = join(ROOT, "public", "sprites");
const OUT_SOUNDS = join(ROOT, "public", "sounds");
const OUT_ICONS = join(ROOT, "public", "icons");

for (const d of [OUT_SPRITES, OUT_SOUNDS, OUT_ICONS]) {
  if (!existsSync(d)) mkdirSync(d, { recursive: true });
}

function run(cmd) {
  console.log(`> ${cmd}`);
  execSync(cmd, { stdio: "inherit" });
}

// Target sprite-sheet geometry (must match src/lib/sprites.ts)
//
// Source video frames are 1080x1920 but the character only occupies
// rows ~308..1633 (1325 tall). We crop the empty vertical padding so the
// sprite's bottom edge sits right at the character's feet — lets us place
// feet directly on the taskbar.
const SRC_W = 1080;
const SRC_H = 1920;
const CROP_Y = 308;
const CROP_H = 1330; // a little safety below feet
const FRAME_W = 225;
// Keep source aspect after crop: 1080/1330 → 225 * (1330/1080) ≈ 278
const FRAME_H = 278;
const FPS = 15;
const COLS = 15;
const ROWS = 10;
const FRAMES = COLS * ROWS; // 150

// Source PNG sequences are ~241 frames at ~24 fps. We use ffmpeg's fps filter
// to resample to 15 fps (giving us ~150 frames over the 10-second clip), then
// colorkey black to transparent, scale, and tile.
//
// colorkey options:
//   0x000000   pure black target
//   0.01       similarity (very tight — only true black)
//   0.05       blend range — softens the edge a few shades above black
function buildSheet(srcDir, dstPng) {
  if (!existsSync(srcDir)) {
    console.warn(`skip: ${srcDir} not found`);
    return;
  }
  const pattern = join(srcDir, "%05d.png");
  run(
    `ffmpeg -y -framerate 24 -start_number 1 -i "${pattern}" ` +
      `-vf "fps=${FPS},crop=${SRC_W}:${CROP_H}:0:${CROP_Y},` +
      `colorkey=0x000000:0.01:0.05,format=rgba,` +
      `scale=${FRAME_W}:${FRAME_H}:flags=lanczos,` +
      `tile=${COLS}x${ROWS}:color=0x00000000" ` +
      `-frames:v 1 "${dstPng}"`,
  );
}

console.log("=== Building sprite-sheet PNGs with transparency ===");
buildSheet(FRAMES_BRUCE, join(OUT_SPRITES, "walk-bruce.png"));
buildSheet(FRAMES_JAZZ, join(OUT_SPRITES, "walk-jazz.png"));
console.log(`Frame meta: ${FRAME_W}x${FRAME_H}, ${COLS} cols, ${ROWS} rows, ${FRAMES} frames @ ${FPS} fps`);

console.log("\n=== Copying sounds ===");
const soundsDir = join(UPSTREAM, "Sounds");
if (existsSync(soundsDir)) {
  for (const f of readdirSync(soundsDir)) {
    copyFileSync(join(soundsDir, f), join(OUT_SOUNDS, f));
    console.log(`copied ${f}`);
  }
}

console.log("\n=== Copying bubble + hang icons ===");
const bubble = join(
  UPSTREAM,
  "Assets.xcassets",
  "MenuBarIcon.imageset",
  "bubble-icon@3x.png",
);
if (existsSync(bubble)) {
  copyFileSync(bubble, join(OUT_ICONS, "bubble.png"));
  console.log("copied bubble.png");
}
const hang = join(
  UPSTREAM,
  "Assets.xcassets",
  "bruce-hang.imageset",
  "bruce-hang.png",
);
if (existsSync(hang)) {
  copyFileSync(hang, join(OUT_SPRITES, "bruce-hang.png"));
  console.log("copied bruce-hang.png");
}

console.log("\nDone.");
