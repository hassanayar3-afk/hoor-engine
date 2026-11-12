#Requires -Version 5.1
<#
.SYNOPSIS
  HOOR Engine v1.0 — إعداد محلي آمن (Windows PowerShell)

.DESCRIPTION
  - ينشئ/يحدّث ملف .env محلياً (لا يُرفع إلى GitHub)
  - يثبّت المتطلبات، يبني المشروع، verify، once
  - اختياري: دفع الكود فقط إلى hassanayar3-afk/hoor-engine

.USAGE
  cd C:\Users\acer\hur
  powershell -ExecutionPolicy Bypass -File .\scripts\HOOR_SETUP_COMPLETE.ps1

  أو: انقر بزر الماوس الأيمن → Run with PowerShell (كمسؤول غير مطلوب)
#>

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $ProjectRoot

function Write-Step($msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Write-Ok($msg)   { Write-Host "OK: $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "WARN: $msg" -ForegroundColor Yellow }

Write-Host @"

  ╔══════════════════════════════════════╗
  ║   HOOR Engine v1.0 — Local Setup     ║
  ║   hassanayar3-afk / hoor-engine      ║
  ╚══════════════════════════════════════╝

"@ -ForegroundColor White

# ─── 1) Rust / Cargo ─────────────────────────────────────────────
Write-Step "التحقق من Rust و Cargo"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo غير مثبت. ثبّت Rust من https://rustup.rs ثم أعد التشغيل."
}
Write-Ok (cargo --version)

# ─── 2) قالب .env ────────────────────────────────────────────────
Write-Step "ملف البيئة .env (محلي — مستثنى من Git)"
$EnvPath = Join-Path $ProjectRoot ".env"
$ExamplePath = Join-Path $ProjectRoot ".env.example"

$CreateNew = -not (Test-Path $EnvPath)
if ($CreateNew) {
    Write-Warn ".env غير موجود — سيتم إنشاؤه."
}

function Read-SecretPlain($Prompt) {
    $sec = Read-Host $Prompt -AsSecureString
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($sec)
    try {
        [Runtime.InteropServices.Marshal]::PtrToStringAuto($bstr)
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
    }
}

$HoorMode = "testnet"
if ($CreateNew -or ($env:HOOR_SETUP_REFRESH_SECRETS -eq "1")) {
    Write-Host @"
اختر بيئة REST:
  [1] Testnet  — testnet.binancefuture.com (مفاتيح من بوابة Testnet)
  [2] Demo     — demo-fapi.binance.com (Demo Trading من باينانس)
  [3] Mainnet  — fapi.binance.com (حي — احذر)
"@ -ForegroundColor Gray
    $pick = Read-Host "اختيار [1/2/3] (افتراضي 1)"
    switch ($pick) {
        "2" { $HoorMode = "demo"; $TestnetFlag = "false" }
        "3" { $HoorMode = "mainnet"; $TestnetFlag = "false" }
        default { $HoorMode = "testnet"; $TestnetFlag = "true" }
    }

    $DryRun = Read-Host "HOOR_DRY_RUN (true=لا أوامر حقيقية) [true]"
    if ([string]::IsNullOrWhiteSpace($DryRun)) { $DryRun = "true" }

    $Symbol = Read-Host "HOOR_SYMBOL [BTCUSDT]"
    if ([string]::IsNullOrWhiteSpace($Symbol)) { $Symbol = "BTCUSDT" }

    $Balance = Read-Host "HOOR_INITIAL_BALANCE_USD [5000]"
    if ([string]::IsNullOrWhiteSpace($Balance)) { $Balance = "5000" }

    Write-Warn "أدخل مفاتيح باينانس لهذه البيئة فقط (لا تُحفظ في GitHub)."
    $ApiKey = Read-Host "HOOR_API_KEY"
    $ApiSecret = Read-SecretPlain "HOOR_API_SECRET (مخفي)"

    Write-Warn "تيليجرام (اختياري — Enter لتخطي)"
    $TgToken = Read-Host "HOOR_TELEGRAM_BOT_TOKEN"
  $TgChat = Read-Host "HOOR_TELEGRAM_CHAT_ID"

    $envContent = @"
# ==========================================
# HOOR Engine v1.0 — تم الإنشاء عبر HOOR_SETUP_COMPLETE.ps1
# لا ترفع هذا الملف إلى GitHub (.gitignore)
# ==========================================

HOOR_MODE=$HoorMode
HOOR_DRY_RUN=$DryRun
HOOR_SYMBOL=$Symbol
HOOR_INITIAL_BALANCE_USD=$Balance

HOOR_API_KEY=$ApiKey
HOOR_API_SECRET=$ApiSecret
HOOR_TESTNET=$TestnetFlag

HOOR_TELEGRAM_BOT_TOKEN=$TgToken
HOOR_TELEGRAM_CHAT_ID=$TgChat

HOOR_WS_FREEZE_MS=15000
HOOR_WS_WARMUP_MS=5000
HOOR_RECV_WINDOW_MS=10000

# تجاوز يدوي للـ REST (اختياري)
# HOOR_REST_BASE=https://testnet.binancefuture.com
"@

    Set-Content -Path $EnvPath -Value $envContent -Encoding UTF8 -NoNewline
    Add-Content -Path $EnvPath -Value "`n"
    Write-Ok "تم حفظ .env في $EnvPath"
} else {
    Write-Ok "ملف .env موجود — لتجديد الأسرار فقط: `$env:HOOR_SETUP_REFRESH_SECRETS='1'` ثم أعد السكربت"
}

# ─── 3) تحقق أمني من Git ─────────────────────────────────────────
Write-Step "فحص أن .env غير متتبع في Git"
if (Test-Path (Join-Path $ProjectRoot ".git")) {
    $tracked = git check-ignore -v .env 2>$null
    if (-not $tracked) {
        throw ".env ليس في .gitignore — أوقف التنفيذ وأصلح .gitignore قبل المتابعة."
    }
    Write-Ok ".env مستثنى من Git"
} else {
    Write-Warn "لا مستودع git محلي — يمكنك تشغيل قسم GitHub لاحقاً"
}

# ─── 4) Build & Test ─────────────────────────────────────────────
Write-Step "cargo test"
cargo test
if ($LASTEXITCODE -ne 0) { throw "cargo test فشل" }
Write-Ok "جميع الاختبارات نجحت"

Write-Step "cargo build --release"
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build فشل" }

# ─── 5) مصادقة باينانس ───────────────────────────────────────────
Write-Step "cargo run -- verify (مصادقة موقّعة)"
cargo run -- verify
$verifyExit = $LASTEXITCODE
if ($verifyExit -ne 0) {
    Write-Warn @"
verify فشل (-2015 شائع): المفتاح لا يطابق البيئة.
  Testnet: أنشئ المفتاح من https://testnet.binancefuture.com
  Demo:    HOOR_MODE=demo و مفاتيح Demo Trading
ثم: cargo run -- verify
"@
} else {
    Write-Ok "مصادقة باينانس ناجحة"
}

# ─── 6) دورة واحدة ───────────────────────────────────────────────
Write-Step "cargo run -- once"
cargo run -- once
if ($LASTEXITCODE -ne 0) { throw "cargo run -- once فشل" }
Write-Ok "دورة once اكتملت"

# ─── 7) رفع GitHub (كود فقط — بدون .env) ─────────────────────────
Write-Step "رفع اختياري إلى GitHub (hassanayar3-afk/hoor-engine)"
$push = Read-Host "هل تريد دفع التغييرات الآمنة إلى origin/main؟ [y/N]"
if ($push -match '^[yY]') {
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        throw "git غير متوفر"
    }
    $stagedEnv = git status --porcelain .env 2>$null
    if ($stagedEnv -match '^[^?]') {
        throw "محاولة تتبع .env — مرفوض. أزل الملف من staging."
    }
    foreach ($f in @(".env", "hoor_trade_journal.json")) {
        if (git ls-files --error-unmatch $f 2>$null) {
            throw "ملف حساس متتبع: $f — استخدم git rm --cached $f"
        }
    }

    $msg = Read-Host "رسالة commit [chore: setup scripts]"
    if ([string]::IsNullOrWhiteSpace($msg)) { $msg = "chore: setup scripts" }

    git add .
    git status
    $confirm = Read-Host "تأكيد commit + push؟ [y/N]"
    if ($confirm -match '^[yY]') {
        git commit -m $msg
        if ($LASTEXITCODE -ne 0) { Write-Warn "لا شيء للالتزام أو فشل commit" }
        else {
            if (-not (git remote get-url origin 2>$null)) {
                git remote add origin https://github.com/hassanayar3-afk/hoor-engine.git
            }
            git push -u origin main
            if ($LASTEXITCODE -eq 0) {
                Write-Ok "https://github.com/hassanayar3-afk/hoor-engine"
            } else {
                Write-Warn "push فشل — جرّب: gh auth login ثم git push -u origin main"
            }
        }
    }
}

Write-Host "`n=== انتهى الإعداد ===" -ForegroundColor Green
Write-Host "تشغيل مستمر: cargo run -- run" -ForegroundColor Gray
Write-Host "توثيق: README_HOOR_PRODUCTION.md" -ForegroundColor Gray
