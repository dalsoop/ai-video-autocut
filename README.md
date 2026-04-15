# autocut-tui

터미널 기반 autocut 클라이언트. Rust + Ratatui + Nickel.

autocut-web HTTP API를 호출해서 **SSH로 접속한 터미널에서** Qwen3-ASR 자막 추출 + 컷 편집을 수행합니다.

## 설치

```bash
cargo build --release
./deploy.sh 50064    # LXC 50064에 배포
pct enter 50064 && autocut-tui
```

## 스크린샷

### 파일 리스트 + 자막 미리보기
파일 커서 위에 250ms 멈추면 해당 파일의 SRT가 우측 패널에 자동 로드됨.

[`docs/screenshots/01-files-with-preview.txt`](docs/screenshots/01-files-with-preview.txt)

```
┌ 파일 (4) ──────────────────────┐┌ 자막 미리보기 ──────────────────┐
│✂ 2026-01-23 19-46-32.mov       ││4 라인 • 4 유지 • 116.5초          │
│  2026-01-23 19-49-24.mov       ││                                   │
│  2026-01-23 20-30-04.mov       ││✓ 00:00 < No Speech >              │
│  2026-01-23 20-56-21.mov       ││✓ 01:44 자 다시 해볼게요           │
│                                ││✓ 01:46 < No Speech >              │
│                                │└───────────────────────────────────┘
│                                │┌ 편집본 (1) ───────────────────────┐
│                                ││2026-01-23 19-46-32.mov (28.0M)    │
│                                │└───────────────────────────────────┘
└────────────────────────────────┘
```

### 자막 편집 뷰
Space로 라인 토글, `a`/`n` 모두 유지/제거, `i` 반전, `c` 컷 실행.

[`docs/screenshots/02-subtitle-editor.txt`](docs/screenshots/02-subtitle-editor.txt)

### 도움말 팝업 (`?`)
[`docs/screenshots/03-help-popup.txt`](docs/screenshots/03-help-popup.txt)

### 컷 확정 다이얼로그
`c` 눌러 컷 실행 전 확인. Y/Enter 실행, N/Esc 취소.

[`docs/screenshots/04-cut-confirm.txt`](docs/screenshots/04-cut-confirm.txt)

## 키바인드 요약

- **파일 뷰:** `j/k` 이동, `Enter` 열기, `/` 검색, `t` 자막추출, `e` 엔진 토글, `l` 언어 토글, `p` 프로젝트
- **자막 뷰:** `Space` 토글, `a/n/i` 전체유지/제거/반전, `c` 컷, `Esc` 뒤로
- **공통:** `?` 도움말, `q` 종료, `Ctrl+C` 강제종료, 진행중에 `Esc`로 취소

`autocut-tui --keys` 전체 목록 출력.

## 설정 (Nickel)

`~/.config/autocut/config.ncl`:

```nickel
{
  endpoint = "http://localhost:8080",
  defaults = {
    engine = "qwen3",
    lang = "Korean",
    whisper_model = "medium",
  },
  keybinds = {
    transcribe = "T",   # 기본 t 덮어쓰기
    cut = "X",
  },
}
```

지원 키 이름: 단일 문자(`"a"`), 공백(`" "`), `"Enter"`, `"Esc"`, `"Tab"`, `"Backspace"`.

## 버전

- v0.1.5 — 자막 미리보기 (2026-04-15)
- v0.1.4 — Nickel keybind 실제 연결, 컷 확인 다이얼로그
- v0.1.3 — 도움말 모달, 파일 검색, 경과시간
- v0.1.2 — Job 취소 (ESC)
- v0.1.1 — Unicode-safe, 뷰포트 스크롤
- v0.1.0 — 초기 스캐폴딩
