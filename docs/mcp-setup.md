# Kuse Cowork MCP 연결 가이드

이 문서는 `Kuse Cowork`에서 현재 연결하려는 MCP 서버들의 설정 방법, 실제 확인 결과, 그리고 현재 한계를 정리한 운영 메모입니다.

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

## 서버별 연결 상태 요약

### GitLab MCP

상태:

- 연결 완료
- 실제 `get_user` 호출 성공 확인

운영 정보:

- Endpoint: `https://mcp.ss-fai.cloud/gitlab/mcp`
- 인증: `Authorization: Bearer <GITLAB_TOKEN>`
- 세션 헤더: `Mcp-Session-Id`

앱 설정:

- `Name`: `GitLab MCP`
- `Remote MCP server URL`: `https://mcp.ss-fai.cloud/gitlab/mcp`
- `Authentication`: `Bearer Token`
- `Bearer Token`: GitLab token
- `Custom Headers (JSON)`: `{}`

비고:

- `Mcp-Session-Id`는 사용자가 직접 입력할 필요가 없습니다.
- 앱이 `initialize` 응답 헤더에서 자동 추출 후 `tools/list`, `tools/call`에 자동 첨부합니다.

실제 확인 결과:

- `get_user` 호출 성공
- 현재 인증 사용자 정보 조회 성공

## Outline MCP

상태:

- 현재 앱에서 직접 연결 가능
- `Bearer Token` 방식으로 연결 가능
- 실제 도구 조회 및 연결 상태 확인

운영 정보:

- Endpoint: `https://wiki.ss-fai.cloud/mcp`
- Authentication: `Bearer Token`

결론:

- Outline MCP는 `Bearer Token` 방식으로 연결 가능
- 앱에서는 일반 Bearer Token MCP 서버와 같은 방식으로 설정하면 됨
- 별도 PKCE 로그인, dynamic client registration, scope 관리 UI는 필요하지 않음

현재 앱에서 시도하지 말아야 하는 방식:

- 불필요한 PKCE/authorization code 설정
- 사용하지 않는 OAuth scope 입력

앱 설정:

- `Name`: `outline`
- `Remote MCP server URL`: `https://wiki.ss-fai.cloud/mcp`
- `Authentication`: `Bearer Token`
- `Bearer Token`: Outline MCP에서 허용하는 토큰 값
- `Custom Headers (JSON)`: 필요 없으면 `{}`

## Mattermost MCP

상태:

- 연결 완료
- 실제 테스트 성공

운영 정보:

- Public endpoint: `https://mcp.ss-fai.cloud/mattermost/mcp`
- Internal endpoint: `http://mcp-proxy:3000/mattermost/mcp`
- Upstream: `http://mattermost-mcp:8000/mcp`
- 프록시가 `Authorization` 헤더를 그대로 전달
- path를 `/mcp`로 rewrite

앱 설정:

- `Name`: `Mattermost MCP`
- `Remote MCP server URL`: `https://mcp.ss-fai.cloud/mattermost/mcp`
- `Authentication`: `Bearer Token`
- `Bearer Token`: Mattermost Personal Access Token
- `Custom Headers (JSON)`: `{}`

전제 조건:

- Mattermost 계정에 `Allow this account to generate personal access tokens` 권한 필요
- `chat.ss-fai.cloud`에서 PAT 발급 필요

비고:

- 외부 경로만 입력하면 됨
- 내부 경로 `http://mcp-proxy:3000/mattermost/mcp`는 컨테이너 내부용

실제 확인 결과:

- 초기 `406 Not Acceptable` 발생
- 원인 1: 서버가 `Accept: application/json, text/event-stream`를 요구
- 원인 2: 일부 응답이 `text/event-stream` 형식으로 반환됨
- 현재는 JSON 우선 요청 후, `406`이면서 `text/event-stream` 요구 메시지가 있으면 자동 재시도
- `text/event-stream` 응답도 파싱하도록 수정
- 최종적으로 연결 및 사용 가능 상태 확인

## 현재 판단 요약

- GitLab MCP: 현재 앱에서 연결 완료
- Mattermost MCP: 현재 앱에서 연결 완료
- Outline MCP: 현재 앱에서 연결 가능

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

- 실제 원격 MCP 서버와의 최종 연동은 각 서버 토큰과 외부 네트워크 상태에 따라 달라질 수 있음
- Outline는 인증 구조상 현재 앱만으로 연결 불가
