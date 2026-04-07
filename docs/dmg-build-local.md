# Kuse Cowork DMG 생성 방법

이 문서는 개발자 또는 운영자가 로컬 Mac에서 `Kuse Cowork`의 macOS 배포 파일을 만드는 방법을 정리한 내부용 문서입니다.

## 결과물

빌드가 성공하면 아래 파일이 생성됩니다.

- `.app`
  `src-tauri/target/release/bundle/macos/Kuse Cowork.app`
- `.dmg`
  `src-tauri/target/release/bundle/dmg/Kuse Cowork_<version>_<arch>.dmg`

예시:

```bash
src-tauri/target/release/bundle/dmg/Kuse Cowork_0.1.0_aarch64.dmg
```

## 사전 준비

필수:

- macOS
- Node.js 18+
- Rust
- Tauri 빌드 환경

권장:

- Docker Desktop
  앱 기능 검증용입니다. DMG 생성 자체에는 필수는 아니지만, 실제 동작 확인에는 필요할 수 있습니다.

## 1. 저장소 준비

```bash
git clone https://github.com/kuse-ai/kuse-cowork.git
cd kuse-cowork
npm install
```

## 2. DMG 빌드

기본 빌드 명령은 아래입니다.

배포본에 현재 내 LLM 설정을 기본값으로 포함하고 싶다면, 빌드 전에 settings preset 파일을 먼저 갱신합니다.

```bash
npm run settings:export-preset
```

이 명령은 로컬 앱 DB에서 현재 모델, base URL, max tokens, temperature 등을 읽어
`src-tauri/default-settings.json`으로 저장합니다.
`api_key`, `provider_keys`는 포함되지 않습니다.

배포본에 현재 내 MCP 서버 목록을 기본값으로 포함하고 싶다면, 빌드 전에 preset 파일을 먼저 갱신합니다.

```bash
npm run mcp:export-presets
```

이 명령은 로컬 앱 DB에서 MCP 서버 목록을 읽어 `src-tauri/default-mcp-servers.json`으로 저장합니다.
`bearer_token`, `oauth_client_secret`, custom header 값 같은 secret은 제외됩니다.

```bash
npm run tauri build
```

이 명령은 다음을 순서대로 수행합니다.

1. 프런트엔드 프로덕션 빌드
2. Rust release 빌드
3. macOS `.app` 번들 생성
4. `.dmg` 파일 생성

## 3. 결과물 확인

```bash
ls -lh src-tauri/target/release/bundle/macos
ls -lh src-tauri/target/release/bundle/dmg
```

정상 빌드라면 `Kuse Cowork.app`와 `.dmg` 파일이 보여야 합니다.

## 4. 실행 확인

로컬에서 앱 번들을 먼저 열어 보고, 그 다음 DMG를 마운트해 설치 흐름을 확인하는 것이 안전합니다.

확인 순서:

1. `src-tauri/target/release/bundle/macos/Kuse Cowork.app` 실행
2. `src-tauri/target/release/bundle/dmg/*.dmg` 더블클릭
3. `/Applications`로 복사 가능한지 확인
4. 복사된 앱이 실제 실행되는지 확인

## 5. 현재 상태에서 주의할 점

현재 저장소는 로컬 빌드 시 DMG를 만들 수 있습니다. 다만 공개 배포용이라면 아래를 추가로 확인해야 합니다.

- Apple Developer ID 서명
- notarization
- Gatekeeper 통과 여부

이 내용은 별도 문서 `[macos-release.md](/Users/samsung/Documents/kuse_cowork/docs/macos-release.md)`에 정리되어 있습니다.

서명 없이 만든 로컬 테스트용 DMG는 아래 같은 경고가 뜰 수 있습니다.

- "확인되지 않은 개발자"
- "손상되었기 때문에 열 수 없습니다"
- 보안 설정에서 수동 허용 필요

즉, 내부 테스트용 DMG와 공개 배포용 DMG는 같은 파일 형식이지만 신뢰 상태가 다를 수 있습니다.

## 6. 자주 쓰는 점검 명령

앱 서명 상태 확인:

```bash
codesign -dv --verbose=4 "src-tauri/target/release/bundle/macos/Kuse Cowork.app"
```

Gatekeeper 판정 확인:

```bash
spctl -a -vv "src-tauri/target/release/bundle/macos/Kuse Cowork.app"
```

## 7. 배포 받은 사용자용 가이드

사용자에게는 이 문서를 보내지 말고 아래 문서를 전달하면 됩니다.

- `[user-install-macos.md](/Users/samsung/Documents/kuse_cowork/docs/user-install-macos.md)`

## 8. Windows 배포 파일 생성은 많이 다른가?

크게 다르지는 않습니다. 같은 Tauri 빌드 파이프라인을 사용하고, 출력 파일 형식만 다릅니다.

공통점:

- 같은 저장소에서 빌드
- 같은 `npm run tauri build` 계열 명령 사용
- 프런트엔드와 Rust 앱을 함께 빌드

차이점:

- macOS는 보통 `.dmg`
- Windows는 보통 `.msi`
- macOS는 Developer ID 서명과 notarization이 중요
- Windows는 code signing 인증서가 있으면 좋지만, notarization 같은 Apple 절차는 없음

현재 릴리스 워크플로우 기준 출력은 아래와 같습니다.

- macOS: `.dmg`
- Windows: `.msi`
- Linux: `.deb`, `.AppImage`

즉, "완전히 다른 방식"은 아니고, 플랫폼별 패키징과 신뢰 체계가 다른 정도로 보면 됩니다.
