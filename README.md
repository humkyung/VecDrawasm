# Vector Draw Webassmebly

### Build
- wasm-pack build --dev --target web : add --dev flag for debugging

### Debugging
- prerequirities
  - Rust Analyzer
  - CodeLLDB
  - Debugger for Chrome
  - rustup target add wasm32-unknown-unknown
- wasm-pack build --target web --debug
- VSCode에서 디버깅 실행
  - VSCode의 디버깅 탭 (Ctrl+Shift+D) 이동.
  - "Debug Rust Wasm in Chrome" 선택 후 "Start Debugging" (F5) 클릭.
  - Chrome이 실행되며 **브라우저 개발자 도구(F12)**에서 wasm 코드 디버깅 가능.