# 교훈 #18 — ort-sys 의존 크레이트는 MSVC C++ 워크로드 필수

## 상황

Phase 62에서 fastembed v5 (BGE-M3 BGE-Reranker-v2-M3 통합 어댑터) 도입. 사전 검증 단계에서 빌드 시도 시 50건의 LNK2019 에러 발생.

## 문제

```
libort_sys-*.rlib(...obj) : error LNK2019: 외부 기호 확인 실패
- _Floating_to_chars_general_precision (MSVC C++17/20 STL)
- __std_max_element_1
- __std_minmax_element_4
fatal error LNK1120: 50개의 확인할 수 없는 외부 참조
```

`cargo check --all`은 통과하던 file-pipeline이 fastembed를 추가하자 빌드 실패. ort-sys의 사전 컴파일된 정적 라이브러리(.lib)가 새 MSVC STL 심볼을 요구하지만, 환경의 cl.exe가 옛 버전이라 미해결.

## 원인

**환경 진단**:
- VS 2022 BuildTools v17.14 설치돼 있었지만 **"Desktop development with C++" 워크로드 미설치**
- VS 2022의 LLVM/Clang만 설치된 상태였음
- 결과적으로 PATH에서 발견된 cl.exe는 **VS 2017 BuildTools v14.16** (옛 버전)
- ort-sys는 MSVC v14.38+ (VS 2022 17.8+) 빌드 산출물이라 v14.16과 STL 심볼 불일치

**근본 원인**: ort-sys (ONNX Runtime Rust 바인딩의 C++ 정적 라이브러리)는 빌드된 MSVC 버전과 호환되는 cl.exe + Windows SDK 필요.

## 개선

### 즉시 처리

1. Visual Studio Installer → VS 2022 BuildTools "수정"
2. **"Desktop development with C++" 워크로드 체크** → 설치 (~2GB)
3. 신규 셸에서 `ls "/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/"` 결과 v14.38+ 확인
4. cargo clean 후 재빌드

### 영향 범위 (재발 방지 체크리스트)

다음 ML 크레이트도 동일 이슈 발생 가능:
- ✅ fastembed (확인됨)
- 🔄 ort, ort-sys (직접 사용 시)
- 🔄 candle (Hugging Face Rust ML)
- 🔄 burn (Rust ML 프레임워크)
- 🔄 onnxruntime-rs

이들 도입 시 사전 점검:

```bash
# MSVC C++ 워크로드 설치 여부 확인
ls "/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/" 2>&1
# v14.38+ 디렉토리 보이면 OK

# 빌드 시 feature 격리 권장
[features]
fastembed = ["dep:fastembed"]  # 기본 빌드는 영향 없게
```

### 문서 갱신

- `spec/architecture.md` Phase 62 섹션에 "VS 2022 Build Tools v17.8+ + C++ 워크로드 필수" 명시
- `prd/roadmap.md` Phase 62 빌드 환경 요구사항 항목 추가
- 향후 README 또는 BUILD.md 작성 시 본 교훈 참조

## 재발 방지

- ML/임베딩 관련 외부 크레이트 도입 시 **사전 검증 단계 1번에서 빌드 시도 필수** — `cargo check`가 아닌 `cargo build`로 링크까지 확인.
- 빌드 환경 의존이 발견되면 **자료/자문에 "런타임 의존" 외에 "빌드 의존"도 별도 명시**할 것 (자문에서도 "런타임 vs 빌드 의존 구분" 제약 사항 추가됨).
- `feature` flag로 격리하여 기본 빌드는 영향받지 않게 — Phase 62 적용 패턴을 표준으로.
