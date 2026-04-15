#!/usr/bin/env python3
"""Qwen3-ASR-1.7B 로컬 GPU transcribe → SRT (공백 보존)"""
import sys, os, time, argparse, torch
from qwen_asr import Qwen3ASRModel

def fmt_ts(sec: float) -> str:
    ms = int(round(sec * 1000))
    h, ms = divmod(ms, 3600000)
    m, ms = divmod(ms, 60000)
    s, ms = divmod(ms, 1000)
    return f"{h:02d}:{m:02d}:{s:02d},{ms:03d}"

def segments(full_text: str, items, max_gap=6.0, max_chars=80):
    """r.text를 원본으로 사용 (공백 보존). aligner items는 타임스탬프만."""
    PUNCT = set(".?!。?!…")
    result = []
    cur_text = ""
    cur_start = None
    cur_end = None
    item_idx = 0

    def emit():
        nonlocal cur_text, cur_start, cur_end
        if cur_text.strip() and cur_start is not None:
            result.append((cur_start, cur_end or cur_start, cur_text.strip()))
        cur_text = ""
        cur_start = None
        cur_end = None

    for c in full_text:
        if c.isspace():
            cur_text += c
            continue
        if c in PUNCT:
            cur_text += c
            emit()
            continue
        # non-space, non-punct: aligner item 소비
        if item_idx < len(items):
            it = items[item_idx]
            # 큰 gap이면 분할
            if cur_start is not None and cur_end is not None and (it.start_time - cur_end) > max_gap:
                emit()
            if cur_start is None:
                cur_start = it.start_time
            cur_end = it.end_time
            # aligner item.text 가 multi-char면 해당 만큼 char 소비
            it_len = max(1, len(it.text))
            item_idx += 1
            # 한 item에 여러 char (영어 단어 등) → 첫 char만 여기서 처리, 나머지는 다음 iter에서 처리할 수 없음
            # 단순화: 1 item = 1 char 가정 (한국어는 대부분 맞음)
        cur_text += c
        if len([x for x in cur_text if not x.isspace() and x not in PUNCT]) >= max_chars:
            emit()
    emit()
    return result

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("input")
    ap.add_argument("--lang", default="Korean")
    ap.add_argument("--device", default="cuda:0")
    ap.add_argument("--asr", default="Qwen/Qwen3-ASR-1.7B")
    ap.add_argument("--aligner", default="Qwen/Qwen3-ForcedAligner-0.6B")
    args = ap.parse_args()

    print(f"[qwen3] loading {args.asr} + {args.aligner} on {args.device}...", flush=True)
    t0 = time.time()
    model = Qwen3ASRModel.from_pretrained(
        args.asr, dtype=torch.float16, device_map=args.device,
        forced_aligner=args.aligner,
        forced_aligner_kwargs={"dtype": torch.float16, "device_map": args.device},
    )
    print(f"[qwen3] loaded in {time.time()-t0:.1f}s", flush=True)

    print(f"[qwen3] transcribing: {args.input}", flush=True)
    t0 = time.time()
    results = model.transcribe(args.input, language=args.lang, return_time_stamps=True)
    print(f"[qwen3] done in {time.time()-t0:.1f}s", flush=True)

    r = results[0]
    if not r.text.strip():
        print("[qwen3] WARN: 빈 결과")
    items = list(r.time_stamps.items) if r.time_stamps else []
    segs = segments(r.text, items) if items else [(0.0, 0.0, r.text.strip())]

    base = os.path.splitext(args.input)[0]
    srt_path = base + ".srt"
    with open(srt_path, "w", encoding="utf-8") as f:
        for i, (s, e, text) in enumerate(segs, 1):
            if not text:
                continue
            f.write(f"{i}\n{fmt_ts(s)} --> {fmt_ts(e)}\n{text}\n\n")
    print(f"[qwen3] SRT written: {srt_path} ({len(segs)} segments)")
    print(f"[qwen3] preview: {r.text[:200]}")

if __name__ == "__main__":
    main()
