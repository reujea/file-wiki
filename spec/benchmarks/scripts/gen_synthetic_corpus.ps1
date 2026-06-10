# gen_synthetic_corpus.ps1 — 5K 규모 합성 코퍼스 생성
#
# 목적: 트리거 #2/#4 + Ruflo A2/B1 디폴트 변경 결정을 위한 측정 코퍼스.
# 실 파일이 부족할 때 doc_type 분포·유사도 분포를 모사한 합성 파일을 만든다.
#
# 사용법:
#   pwsh -File spec/benchmarks/scripts/gen_synthetic_corpus.ps1 -OutDir D:\file-test\synthetic_5k -Count 5000
#
# 분포 (다축 프로파일 가정 — content_mix 모사):
#   meeting     30% (회의록, 의사결정 패턴)
#   research    20% (장문, 인용·키워드 다수)
#   code         15% (코드 블록, 짧은 메모)
#   legal        10% (긴 문서, 형식 일정)
#   general      25% (일반 메모, 짧음)

param(
    [Parameter(Mandatory=$true)]
    [string]$OutDir,
    [int]$Count = 5000,
    [switch]$Force
)

if (Test-Path $OutDir) {
    if ($Force) {
        Remove-Item -Recurse -Force $OutDir
    } else {
        Write-Error "$OutDir 이미 존재. -Force로 덮어쓰기."
        exit 1
    }
}
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

# 분포 (퍼센트 → 누적 임계값)
$dist = @(
    @{ Type='meeting';  Pct=30 },
    @{ Type='research'; Pct=20 },
    @{ Type='code';     Pct=15 },
    @{ Type='legal';    Pct=10 },
    @{ Type='general';  Pct=25 }
)

# 키워드 풀 (doc_type별, 50개씩 — 유사도 클러스터링 효과)
$keywords = @{
    meeting  = @('회의록','결정','액션','담당자','마감','참석','의제','후속','TODO','승인','보류','반대',
                 '이슈','블로커','이번주','다음주','분기','목표','리뷰','발표','데모','피드백',
                 '클라이언트','내부','외부','파트너','계약','일정','우선순위','MTG','킥오프')
    research = @('가설','실험','검증','데이터셋','모델','정확도','벤치마크','baseline','SOTA','MRR',
                 'F1','recall','precision','임베딩','벡터','코사인','dim','token','context','attention',
                 'transformer','BGE','M3','sparse','dense','reranker','크로스인코더','HNSW')
    code     = @('함수','클래스','모듈','impl','trait','async','await','Result','Error','unwrap',
                 'expect','match','if let','clone','Arc','Mutex','Vec','HashMap','build','test',
                 'cargo','rustc','clippy','nextest','workspace','feature','dep','crate','PR')
    legal    = @('계약','조항','당사자','책임','면책','보증','위반','해지','갱신','자동연장',
                 '관할','준거법','중재','분쟁','손해배상','지급','대금','기한','종료','효력',
                 '서명','날인','발효','수정','변경','동의','승인','거절','조건','부속서')
    general  = @('일정','메모','진행','상태','확인','요청','검토','전달','참고','공유','정리',
                 '리스트','체크','업데이트','문의','답변','OK','대기','확인필요','보류','TBD')
}

# Lorem 문장 풀 — 한국어 자연 문장
$loremKo = @(
    '이번 주에는 핵심 기능 안정화에 집중한다.',
    '담당자와 일정은 회의록 마지막 섹션에 정리되어 있다.',
    '벤치마크 결과는 이전 베이스라인 대비 3.5배 빠르다.',
    '환경 의존성으로 인해 측정값에 ±10% 변동이 있다.',
    '리뷰어는 PR description에 테스트 시나리오를 명시할 것.',
    '계약 효력은 양 당사자 서명일로부터 발생한다.',
    '회의 결과는 문서 작성자가 24시간 내 공유한다.',
    '분기 목표는 OKR 시스템에 등록 후 진행 상황을 매주 갱신한다.',
    '실험은 동일 환경에서 3회 반복 후 중앙값을 채택한다.',
    '버전 호환성은 SemVer를 따른다.'
)

function New-Doc {
    param([string]$Type, [int]$Index)
    $kw = $keywords[$Type]
    $picks = @()
    for ($i = 0; $i -lt 12; $i++) {
        $picks += $kw[(Get-Random -Maximum $kw.Count)]
    }

    $bodyLines = @()
    $bodyLines += "# $Type 문서 #$Index"
    $bodyLines += ""

    # 문서 길이는 type별로 다르게
    $sentenceCount = switch ($Type) {
        'meeting'  { Get-Random -Minimum 15 -Maximum 35 }
        'research' { Get-Random -Minimum 40 -Maximum 80 }
        'code'     { Get-Random -Minimum 8  -Maximum 20 }
        'legal'    { Get-Random -Minimum 30 -Maximum 60 }
        default    { Get-Random -Minimum 5  -Maximum 15 }
    }

    for ($s = 0; $s -lt $sentenceCount; $s++) {
        $sentence = $loremKo[(Get-Random -Maximum $loremKo.Count)]
        # 키워드 1~3개 삽입
        $inject = (Get-Random -Minimum 1 -Maximum 4)
        for ($j = 0; $j -lt $inject; $j++) {
            $sentence += " " + $picks[(Get-Random -Maximum $picks.Count)]
        }
        $bodyLines += $sentence
    }

    if ($Type -eq 'code') {
        $bodyLines += ""
        $bodyLines += '```rust'
        $bodyLines += 'fn main() -> Result<()> {'
        $bodyLines += '    let cfg = load_config()?;'
        $bodyLines += '    run(cfg)'
        $bodyLines += '}'
        $bodyLines += '```'
    }

    return ($bodyLines -join "`n")
}

# 누적 분포 계산
$cum = 0
$thresholds = @()
foreach ($d in $dist) {
    $cum += $d.Pct
    $thresholds += @{ Type=$d.Type; Cum=$cum }
}

$created = @{}
foreach ($d in $dist) { $created[$d.Type] = 0 }

$start = Get-Date
for ($i = 1; $i -le $Count; $i++) {
    $r = Get-Random -Minimum 1 -Maximum 101
    $type = 'general'
    foreach ($t in $thresholds) {
        if ($r -le $t.Cum) { $type = $t.Type; break }
    }
    $created[$type]++

    $content = New-Doc -Type $type -Index $i
    $path = Join-Path $OutDir ("{0}_{1:D5}.md" -f $type, $i)
    Set-Content -Path $path -Value $content -Encoding UTF8

    if ($i % 500 -eq 0) {
        $elapsed = (Get-Date) - $start
        Write-Host ("[{0}/{1}] elapsed {2:N1}s" -f $i, $Count, $elapsed.TotalSeconds)
    }
}

$elapsed = (Get-Date) - $start
Write-Host ""
Write-Host "=== 합성 코퍼스 생성 완료 ==="
Write-Host ("총 {0}건 / {1:N1}초 ({2:N1} 파일/초)" -f $Count, $elapsed.TotalSeconds, ($Count / $elapsed.TotalSeconds))
Write-Host ""
Write-Host "분포:"
foreach ($k in $created.Keys) {
    $pct = [math]::Round(100 * $created[$k] / $Count, 1)
    Write-Host ("  {0,-10} {1,5}건 ({2}%)" -f $k, $created[$k], $pct)
}
Write-Host ""
Write-Host "다음 단계:"
Write-Host "  1. pipeline.exe inbox 폴더로 일부 또는 전체 복사"
Write-Host "  2. pipeline.exe process (또는 watcher 자동 트리거)"
Write-Host "  3. cargo test --test bench_real_corpus -- --nocapture (트리거 #2/#4 측정)"
