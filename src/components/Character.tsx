import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CHARACTERS,
  COMPLETION_PHRASES,
  COMPLETION_SOUNDS,
  DISPLAY_HEIGHT,
  DISPLAY_WIDTH,
  PAUSE_RANGE_SEC,
  REFERENCE_WIDTH,
  SHEET_COLS,
  SHEET_FPS,
  SHEET_FRAMES,
  SHEET_ROWS,
  THINKING_PHRASES,
  WALK_AMOUNT_RANGE,
  WALK_TIMING,
  movementPosition,
  randomPhrase,
  type CharacterId,
} from "../lib/sprites";
import { appStore, useAppStore } from "../store/appStore";

interface Props {
  character: CharacterId;
  initialFraction: number;
  trackLeft: number;
  trackRight: number;
  trackBottom: number;
  onClick: () => void;
}

type Phase = "pausing" | "walking";

function randBetween(a: number, b: number) {
  return a + Math.random() * (b - a);
}

export default function Character(props: Props) {
  const { character, initialFraction, trackLeft, trackRight, trackBottom, onClick } = props;
  const cfg = CHARACTERS[character];

  const spriteRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const hitboxRef = useRef<HTMLDivElement>(null);

  const aiStatus = useAppStore((s) => s.aiStatus[character]);
  const aiStatusRef = useRef(aiStatus);
  useEffect(() => {
    aiStatusRef.current = aiStatus;
  }, [aiStatus]);

  const [bubbleText, setBubbleText] = useState<string>("");
  const [bubbleIsCompletion, setBubbleIsCompletion] = useState(false);

  useEffect(() => {
    const sprite = spriteRef.current;
    const container = containerRef.current;
    if (!sprite || !container) return;

    const halfW = DISPLAY_WIDTH / 2;
    const minCx = trackLeft + halfW;
    const maxCx = trackRight - halfW;
    const trackWidth = Math.max(maxCx - minCx, 1);

    let positionProgress = Math.max(0, Math.min(1, initialFraction));
    let goingRight = Math.random() > 0.5;
    let phase: Phase = "pausing";
    let pauseEnd = performance.now() / 1000 + randBetween(1.0, 3.0);
    let walkStartTime = 0;
    let walkStartPixel = 0;
    let walkEndPixel = 0;
    let spriteStartTime = performance.now() / 1000;

    let currentPhrase = "";
    let lastPhraseUpdate = 0;
    let nextPhraseChange = 0;
    let completionExpiry = 0;
    let showingCompletion = false;
    let lastAiStatus = aiStatusRef.current;

    const startWalk = (now: number) => {
      phase = "walking";
      walkStartTime = now;
      spriteStartTime = now;
      if (positionProgress > 0.85) goingRight = false;
      else if (positionProgress < 0.15) goingRight = true;
      else goingRight = Math.random() > 0.5;

      const walkPixels = randBetween(WALK_AMOUNT_RANGE[0], WALK_AMOUNT_RANGE[1]) * REFERENCE_WIDTH;
      const walkAmount = trackWidth > 0 ? walkPixels / trackWidth : 0.3;
      const walkStart = positionProgress;
      const walkEnd = goingRight
        ? Math.min(walkStart + walkAmount, 1)
        : Math.max(walkStart - walkAmount, 0);

      walkStartPixel = walkStart * trackWidth;
      walkEndPixel = walkEnd * trackWidth;
    };

    const enterPause = (now: number) => {
      phase = "pausing";
      pauseEnd = now + randBetween(PAUSE_RANGE_SEC[0], PAUSE_RANGE_SEC[1]);
    };

    let rafId = 0;
    const loop = () => {
      const now = performance.now() / 1000;

      if (phase === "pausing") {
        if (now >= pauseEnd) startWalk(now);
      } else {
        const elapsed = now - walkStartTime;
        const videoTime = Math.min(elapsed, WALK_TIMING.videoDuration);
        const walkNorm = elapsed >= WALK_TIMING.videoDuration ? 1 : movementPosition(videoTime);
        const pixel = walkStartPixel + (walkEndPixel - walkStartPixel) * walkNorm;
        positionProgress = Math.max(0, Math.min(1, pixel / trackWidth));

        if (elapsed >= WALK_TIMING.videoDuration) enterPause(now);
      }

      // Sprite frame — pausing shows frame 0, walking cycles through the walk window
      // (the video's walk cycle runs between accelStart and walkStop seconds)
      let frameIdx = 0;
      if (phase === "walking") {
        const walkElapsed = now - spriteStartTime;
        frameIdx = Math.floor(walkElapsed * SHEET_FPS) % SHEET_FRAMES;
      }
      const col = frameIdx % SHEET_COLS;
      const row = Math.floor(frameIdx / SHEET_COLS);
      sprite.style.backgroundPosition = `-${col * DISPLAY_WIDTH}px -${row * DISPLAY_HEIGHT}px`;

      const cx = minCx + positionProgress * trackWidth;
      const left = cx - halfW;
      const top = trackBottom - DISPLAY_HEIGHT;
      container.style.transform = `translate(${left}px, ${top}px)`;
      if (hitboxRef.current) {
        hitboxRef.current.style.transform = `translate(${left}px, ${top}px)`;
      }

      const want = goingRight ? "" : "scaleX(-1)";
      if (sprite.style.transform !== want) sprite.style.transform = want;

      const bounds = {
        x: left,
        y: top,
        w: DISPLAY_WIDTH,
        h: DISPLAY_HEIGHT,
      };
      appStore.setBounds(character, bounds);
      // Rust cursor-polling uses these bounds to decide when to flip the
      // overlay window out of click-through mode.
      invoke("report_bounds", { character, bounds }).catch(() => {});

      // Bubble state machine
      const currentAi = aiStatusRef.current;
      if (currentAi === "completed" && lastAiStatus !== "completed") {
        const phrase = randomPhrase(COMPLETION_PHRASES);
        currentPhrase = phrase;
        showingCompletion = true;
        completionExpiry = now + 3.0;
        setBubbleText(phrase);
        setBubbleIsCompletion(true);
        playCompletionSound();
      } else if (showingCompletion) {
        if (now >= completionExpiry) {
          showingCompletion = false;
          setBubbleText("");
        }
      } else if (currentAi === "busy") {
        if (currentPhrase === "" || now - lastPhraseUpdate > nextPhraseChange) {
          currentPhrase = randomPhrase(THINKING_PHRASES, currentPhrase);
          lastPhraseUpdate = now;
          nextPhraseChange = randBetween(3.0, 5.0);
          setBubbleText(currentPhrase);
          setBubbleIsCompletion(false);
        }
      } else if (bubbleText !== "" && currentAi === "idle" && !showingCompletion) {
        currentPhrase = "";
        setBubbleText("");
      }
      lastAiStatus = currentAi;

      rafId = requestAnimationFrame(loop);
    };

    rafId = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(rafId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character, trackLeft, trackRight, trackBottom, initialFraction]);

  const sheetW = SHEET_COLS * DISPLAY_WIDTH;
  const sheetH = SHEET_ROWS * DISPLAY_HEIGHT;

  return (
    <>
      <div
        ref={containerRef}
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          width: DISPLAY_WIDTH,
          height: DISPLAY_HEIGHT,
          willChange: "transform",
          pointerEvents: "none",
        }}
      >
        <div
          ref={spriteRef}
          style={{
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
            backgroundImage: `url(${cfg.sheetSrc})`,
            backgroundRepeat: "no-repeat",
            backgroundSize: `${sheetW}px ${sheetH}px`,
            imageRendering: "auto",
          }}
        />
        {bubbleText && (
          <div
            style={{
              position: "absolute",
              top: -8,
              left: "50%",
              transform: "translateX(-50%)",
              padding: "4px 12px",
              background: "rgba(24,24,28,0.92)",
              color: bubbleIsCompletion ? "#7CFFB2" : "#FFFFFF",
              border: `1px solid ${bubbleIsCompletion ? "#3AA76A" : "rgba(255,255,255,0.25)"}`,
              borderRadius: 12,
              fontFamily: "-apple-system, system-ui, sans-serif",
              fontSize: 12,
              fontWeight: 500,
              whiteSpace: "nowrap",
              pointerEvents: "none",
              boxShadow: "0 2px 8px rgba(0,0,0,0.4)",
            }}
          >
            {bubbleText}
          </div>
        )}
      </div>
      <div
        ref={hitboxRef}
        onClick={onClick}
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          width: DISPLAY_WIDTH,
          height: DISPLAY_HEIGHT,
          cursor: "pointer",
          pointerEvents: "auto",
          willChange: "transform",
        }}
        title={character}
      />
    </>
  );
}

let lastSoundIdx = -1;
function playCompletionSound() {
  let idx = Math.floor(Math.random() * COMPLETION_SOUNDS.length);
  if (idx === lastSoundIdx && COMPLETION_SOUNDS.length > 1) {
    idx = (idx + 1) % COMPLETION_SOUNDS.length;
  }
  lastSoundIdx = idx;
  const audio = new Audio(COMPLETION_SOUNDS[idx]);
  audio.volume = 0.5;
  audio.play().catch(() => {});
}
