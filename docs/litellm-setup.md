# Kuse Cowork에서 LiteLLM 사용하기

이 문서는 `Kuse Cowork`를 `LiteLLM` 또는 기타 OpenAI 호환 엔드포인트와 연결하는 방법을 정리한 가이드입니다.

현재 기준으로 아래와 같은 OpenAI 호환 호출이 이미 동작하는 환경을 대상으로 합니다.

```bash
curl https://llm.ss-fai.cloud/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "Qwen3.5-397B-A17B-FP8",
    "messages": [
      {"role": "user", "content": "안녕하세요"}
    ]
  }'
```

## 전제 조건

- `kuse_cowork`가 로컬에 설치되어 있어야 합니다.
- LiteLLM 또는 OpenAI 호환 API 서버가 실행 중이어야 합니다.
- 아래 항목을 알고 있어야 합니다.
  - Base URL
  - API Key
  - Model ID

예시:

- Base URL: `https://llm.ss-fai.cloud/v1`
- API Key: `YOUR_API_KEY`
- Model ID: `Qwen3.5-397B-A17B-FP8`

## 앱 실행

프로젝트 루트에서 실행합니다.

```bash
cd /Users/samsung/Documents/kuse_cowork
npm run tauri dev
```

## 앱 설정 방법

앱이 열리면 `Settings`에서 아래처럼 입력합니다.

### 1. Model Selection

- `Custom` 선택

### 2. Model ID

아래 값을 입력합니다.

```text
Qwen3.5-397B-A17B-FP8
```

### 3. API Configuration

- `API Base URL`

```text
https://llm.ss-fai.cloud/v1
```

- `API Key`

```text
YOUR_API_KEY
```

설정 후 `Test Connection`을 눌러 연결을 확인합니다.

## 왜 Base URL을 `/v1`까지 넣어야 하나요?

Kuse Cowork는 OpenAI 호환 엔드포인트를 호출할 때 다음 규칙을 사용합니다.

- Base URL이 `/v1`로 끝나면: `{base_url}/chat/completions`
- Base URL이 `/v1`로 끝나지 않으면: `{base_url}/v1/chat/completions`

따라서 아래 둘 중 하나로 입력해야 최종 URL이 원하는 값이 됩니다.

- `https://llm.ss-fai.cloud/v1`
- 또는 `/v1`이 없는 서버라면 그에 맞는 상위 URL

이번 사례에서는 최종 요청 URL이 아래처럼 되어야 정상입니다.

```text
https://llm.ss-fai.cloud/v1/chat/completions
```

## 이전에 발생했던 오류

다음 오류가 발생할 수 있었습니다.

```text
Error: API error: {"detail":"Not Found"}
```

원인은 `Qwen3.5-397B-A17B-FP8` 같은 커스텀 모델명이 내부에서 잘못 추론되어 `Anthropic` 방식으로 처리되었기 때문입니다.

그 경우 요청이 OpenAI 호환 경로가 아니라 다른 경로로 전송되어 `404 Not Found`가 발생할 수 있었습니다.

## 이번 수정 사항

LiteLLM 같은 OpenAI 호환 커스텀 엔드포인트를 안정적으로 사용할 수 있도록 아래 내용을 수정했습니다.

### 변경 내용

- 알 수 없는 모델명은 기본적으로 `custom` provider로 처리
- `base_url`을 기준으로 provider를 먼저 추론
- 공식 API가 아닌 다른 URL은 `custom`으로 처리
- `Test Connection`에서도 `custom` provider를 유지한 채 OpenAI 호환 방식으로 호출

### 수정된 파일

- [src/stores/settings.ts](/Users/samsung/Documents/kuse_cowork/src/stores/settings.ts)
- [src-tauri/src/database.rs](/Users/samsung/Documents/kuse_cowork/src-tauri/src/database.rs)
- [src-tauri/src/commands.rs](/Users/samsung/Documents/kuse_cowork/src-tauri/src/commands.rs)

## 권장 입력 예시

LiteLLM에 아래처럼 연결하면 됩니다.

```text
Model Selection: Custom
Model ID: Qwen3.5-397B-A17B-FP8
API Base URL: https://llm.ss-fai.cloud/v1
API Key: YOUR_API_KEY
```

## 문제 해결

### `Not Found`가 다시 뜨는 경우

- `API Base URL`이 정확히 `https://llm.ss-fai.cloud/v1`인지 확인
- 서버가 실제로 `/v1/chat/completions`를 지원하는지 확인
- `Model ID` 오타가 없는지 확인

### 인증 오류가 나는 경우

- `API Key`가 정확한지 확인
- 서버가 `Authorization: Bearer ...` 형식을 사용하는지 확인

### curl은 되는데 앱에서 안 되는 경우

아래 3개가 curl과 정확히 같은지 비교합니다.

- Base URL
- Authorization 방식
- Model ID

## 검증 방법

연결 확인은 아래 순서로 진행하는 것을 권장합니다.

1. curl로 먼저 성공 확인
2. 앱에서 `Custom` + 동일한 `Base URL` + 동일한 `Model ID` 입력
3. `Test Connection` 성공 확인
4. 실제 Task 또는 Chat 실행

## 참고

이 설정 방식은 LiteLLM뿐 아니라 OpenAI 호환 API를 제공하는 다른 프록시나 게이트웨이에도 동일하게 적용할 수 있습니다.
