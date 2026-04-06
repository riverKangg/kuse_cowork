# Kuse Cowork MCP 연결 가이드

이 문서는 `Kuse Cowork`의 MCP 연결 방식과 공개 가능한 설정 예시만 정리합니다. 실제 운영용 엔드포인트, 내부 프록시 구조, 발급 절차, 토큰 값은 저장소 밖의 비공개 운영 문서에서 관리해야 합니다.

기준일:

- 2026-04-06

## 현재 구현 상태

`Kuse Cowork`는 현재 원격 HTTP MCP 서버 연결을 지원하며, 아래 인증 방식을 처리합니다.

- 인증 없음
- Bearer token
- OAuth client credentials
- Custom headers(JSON)

추가로 다음 동작을 지원하도록 수정했습니다.

- `Authorization: Bearer ...` 헤더 자동 전송
- `Mcp-Session-Id` 응답 헤더 저장 후 후속 요청에 자동 전송
- OAuth metadata discovery 일부 지원
- Outline 스타일의 `/.well-known/.../mcp` 경로 탐색 지원

## 앱에서 입력 가능한 MCP 설정값

MCP Settings 화면에서 서버별로 아래 항목을 입력할 수 있습니다.

- `Name`
- `Remote MCP server URL`
- `Authentication`
  - `None`
  - `Bearer Token`
  - `OAuth Client Credentials`
- `Bearer Token`
- `OAuth Client ID`
- `OAuth Client Secret`
- `Custom Headers (JSON)`

## 서버별 연결 예시

다음 예시는 공개 저장소에 남겨도 되는 범위만 포함합니다.

### Bearer Token 기반 MCP

앱 설정:

- `Name`: 서비스 식별용 이름
- `Remote MCP server URL`: 서버 운영자가 제공한 HTTPS MCP URL
- `Authentication`: `Bearer Token`
- `Bearer Token`: 서버 운영자가 발급한 토큰
- `Custom Headers (JSON)`: 필요 없으면 `{}`

비고:

- `Mcp-Session-Id`는 사용자가 직접 입력할 필요가 없습니다.
- 앱이 `initialize` 응답 헤더에서 자동 추출 후 후속 요청에 자동 첨부합니다.

### OAuth Client Credentials 기반 MCP

앱 설정:

- `Name`: 서비스 식별용 이름
- `Remote MCP server URL`: 서버 운영자가 제공한 HTTPS MCP URL
- `Authentication`: `OAuth Client Credentials`
- `OAuth Client ID`: 서버 운영자가 발급한 클라이언트 ID
- `OAuth Client Secret`: 서버 운영자가 발급한 클라이언트 시크릿
- `Custom Headers (JSON)`: 필요할 때만 입력

비고:

- 앱은 metadata discovery를 먼저 시도하고, 실패 시 기본 토큰 경로를 사용합니다.

### SSE 응답을 요구하는 MCP

일부 서버는 `Accept: application/json, text/event-stream` 조합을 요구할 수 있습니다.

현재 동작:

- 먼저 JSON 우선 요청을 보냅니다.
- 서버가 `406 Not Acceptable`과 함께 `text/event-stream` 요구를 반환하면 자동 재시도합니다.
- `text/event-stream` 응답의 JSON payload도 파싱합니다.

## 구현 변경 사항

이번 작업에서 아래 변경을 적용했습니다.

- MCP 설정에 `auth_type`, `bearer_token`, `custom_headers` 추가
- SQLite MCP 저장 스키마 확장 및 기존 DB 마이그레이션 처리
- HTTP MCP 클라이언트에 Bearer token 및 custom headers 적용
- OAuth token 처리 시 `Bearer Bearer ...` 버그 수정
- Outline용 OAuth metadata discovery 경로 보강
- Mattermost용 `Accept` 헤더 fallback 추가
- Mattermost용 `text/event-stream` 응답 파싱 추가
- MCP Settings UI에 인증 타입 선택 및 관련 입력 필드 추가

관련 파일:

- [src-tauri/src/mcp/types.rs](/Users/samsung/Documents/kuse_cowork/src-tauri/src/mcp/types.rs)
- [src-tauri/src/mcp/storage.rs](/Users/samsung/Documents/kuse_cowork/src-tauri/src/mcp/storage.rs)
- [src-tauri/src/mcp/client.rs](/Users/samsung/Documents/kuse_cowork/src-tauri/src/mcp/client.rs)
- [src-tauri/src/mcp/http_client.rs](/Users/samsung/Documents/kuse_cowork/src-tauri/src/mcp/http_client.rs)
- [src/components/MCPSettings.tsx](/Users/samsung/Documents/kuse_cowork/src/components/MCPSettings.tsx)
- [src/lib/mcp-api.ts](/Users/samsung/Documents/kuse_cowork/src/lib/mcp-api.ts)

## 검증 결과

로컬 검증:

- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run build`

결과:

- 모두 통과

주의:

- 실제 원격 MCP 서버와의 최종 연동은 서버 설정, 토큰 권한, 외부 네트워크 상태에 따라 달라질 수 있습니다.
- 운영용 URL, 내부 서비스명, 프록시 구조, 토큰 발급 절차는 공개 저장소에 기록하지 않는 편이 안전합니다.
