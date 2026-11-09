# HOOR Engine v1.0 — دليل الإنتاج والتشغيل

**حُر (HOOR Engine)** — محرك تداول سيادي موحّد (Unified Sovereign Monolith) لعقود باينانس **USDⓈ-M Futures**، مبني بلغة **Rust**، بمسار قرار خطي واحد وطبقة بورصة منفصلة عن منطق القرار.

| البند | القيمة |
|--------|--------|
| الحزمة | `hoor-engine` v1.0.0 |
| المنصة | Binance Futures (mainnet / testnet) |
| حالة الجودة | **24/24** اختبار وحدة (`cargo test`) |
| المرجع الهندسي | `الفهرس.md` |

---

## 1. نظرة معمارية

### 1.1 النموذج: Unified Sovereign Monolith

كل منطق التداول الحاكم يعيش داخل كتلة Rust واحدة (`hoor-engine`). لا حلقات قرار متنافسة، ولا تخزين معقد مشترك بين عمليات متوازية. الحالة الحرجة (صفقة مفتوحة، طوارئ، رصيد، أوزان API) مركزية في `HoorTradingEngine`.

### 1.2 Single Linear Pipeline

البوت **لا يتخذ قرارين في آن واحد**. التسلسل ثابت وقابل للتدقيق:

```text
المستوى 0 — ماكرو + درع الأوزان + طوارئ
    ↓
المستوى 1 — إجماع الزخم + محاذاة CVD + حالة UnifiedMarketState
    ↓
المستوى 2 — عمق السيولة (5× حجم الأمر الاسمي)
    ↓
المستوى 3 — موافقة الهجوم (التنفيذ في Runner؛ القفل الذري بعد نجاح الأمر)
```

| الطبقة | المسؤولية | مسارات رئيسية |
|--------|-----------|----------------|
| **types** | لقطة سوق `UnifiedMarketState` (`repr(C)`) | `src/types/market_state.rs` |
| **guards** | وقت السيرفر + سقف وزن محلي 4000/دقيقة | `guards/chrono.rs`, `guards/rate_limit.rs` |
| **engine** | قرار، Kelly، خروج مرحلي، إشارة، جورنال | `engine/decision.rs`, `signal.rs`, `kelly.rs`, `exit.rs`, `journal.rs` |
| **validation** | عمق دفتر الأوامر | `validation/depth.rs` |
| **pipeline** | حلقة سيادية واحدة | `pipeline/loop_runner.rs` |
| **exchange** | REST، WebSocket، تيليجرام — **بدون** منطق قرار | `exchange/*` |
| **runner** | ربط البورصة بالحلقة، دخول/خروج، إقلاع | `runner/cycle.rs`, `startup.rs`, `exit.rs` |

### 1.3 تدفق البيانات الحيّ

```text
  WebSocket (bookTicker + aggTrade)
           │
           ▼
     MarketFeed ──► mid + cvd_h1_scaled
           │
           ▼
  REST (depth, ticker 24h, ATR 1h, موقّع عند الحاجة)
           │
           ▼
     UnifiedMarketState ──► CycleInput ──► execute_main_cycle
```

- **السعر المعدّل**: mid من `bookTicker` (مع احتياط REST عند الحاجة بعد Warm-up).
- **CVD الساعة**: تجميع حي من `aggTrade` (إعادة ضبط كل ساعة UTC)، وليس تقريباً من شموع REST.
- **الإجماع الكامل**: `engine/signal.rs` — طبقة التيكر في الحقل الموحّد + دمج داخل القرار.

### 1.4 الصفقة الوحيدة (Single-Position Atomic Lock)

- `is_position_active` يُفعَّل **بعد** نجاح أمر الدخول في `runner/cycle.rs` (`execute_attack`).
- الخروج عبر `engine/exit.rs` (TP1/TP2/TP3 trailing + SL) و`runner/exit.rs` (تنفيذ أوامر `reduceOnly`).
- `release_position_lock` عند إغلاق المركز أو عدم وجود مركز/أوامر على المنصة.

---

## 2. ملخص الإصلاحات الحرجة (Production Hardening)

| الإصلاح | الهدف | التنفيذ |
|---------|--------|---------|
| **تبريد تيليجرام (Manual)** | منع إزعاج نطاق 80–84% | 15 دقيقة لكل رمز — `send_manual_alert` |
| **تنبيهات طوارئ تيليجرام** | رصد فوري للمخاطر | `send_critical_alert` **بدون تبريد** |
| **القفل الذري** | عدم قفل الصفقة قبل تنفيذ ناجح | القفل في Runner بعد `place_market` OK |
| **minNotional** | أوامر جزئية TP لا تُرفض صامتاً | `resolve_close_qty_notional` في `runner/exit.rs` |
| **استعادة `tp_stage`** | عدم إعادة العداد عند إعادة التشغيل | `infer_tp_stage` في `startup_recovery` |
| **Kelly ديناميكي** | حجم يعكس الأداء الفعلي | `TradeJournal` → `hoor_trade_journal.json` |
| **CVD حي** | مؤشر حقيقي لا proxy REST | `MarketFeed` + `run_market_stream` |
| **قرار على الحالة الموحدة** | لا حقول شكلية | `decision.rs` يقرأ lock/price/CVD/إجماع |
| **Warm-up إقلاع** | إنهاء سباق WS الأولى | `wait_for_live_ready` حتى 5 ثوانٍ (قابل للتعديل) |
| **قفل مع أوامر مفتوحة** | أمان عند استئناف | `startup_recovery` + `has_open_orders` |

### عتبات تشغيلية

| الثابت | القيمة | ملاحظة |
|--------|--------|--------|
| عتبة إجماع الهجوم | 85_000_000 (×10⁶) | `config::CONSENSUS_ATTACK_THRESHOLD` |
| نطاق التنبيه اليدوي | 80–84% | `ManualAlert` فقط |
| تجميد WS | `HOOR_WS_FREEZE_MS` (افتراضي 8000) | watchdog |
| Warm-up WS | `HOOR_WS_WARMUP_MS` (افتراضي 5000) | قبل أول دورة في `once`/`run` |
| تنبيه أخطاء بورصة متتالية | > 3 | تيليجرام حرج |
| `emergency_halt` | ≥ 5 أخطاء متتالية | يوقف الدخول |

---

## 3. دليل التشغيل السريع

### 3.1 المتطلبات

- Rust toolchain (إصدار workspace في `Cargo.toml` — edition 2021).
- اتصال إنترنت مستقر إلى باينانس وTelegram API.
- ملف `.env` في جذر المشروع (انظر `.env.example`).

### 3.2 إعداد `.env`

انسخ القالب:

```bash
cp .env.example .env
```

| المتغير | مطلوب لـ | الوصف |
|---------|----------|--------|
| `HOOR_API_KEY` / `BINANCE_API_KEY` | أوامر حقيقية، رصيد، مركز | مفتاح API |
| `HOOR_API_SECRET` / `BINANCE_API_SECRET` | نفس ما سبق | سر التوقيع HMAC |
| `HOOR_DRY_RUN` | — | `true` (افتراضي): لا أوامر حية؛ `false` للتنفيذ الفعلي |
| `HOOR_TESTNET` | اختبار | `true` لعناوين testnet |
| `HOOR_SYMBOL` | التداول | افتراضي `BTCUSDT` |
| `HOOR_BTC_SYMBOL` | صدمة ماكرو | افتراضي `BTCUSDT` |
| `HOOR_TELEGRAM_BOT_TOKEN` | التنبيهات | بديل: `TELEGRAM_BOT_TOKEN` |
| `HOOR_TELEGRAM_CHAT_ID` | التنبيهات | بديل: `TELEGRAM_CHAT_ID` |
| `HOOR_WS_FREEZE_MS` | watchdog | مهلة صمت التدفق |
| `HOOR_WS_WARMUP_MS` | إقلاع | انتظار أول `bookTicker` |

**تحميل تلقائي:** `dotenvy::dotenv()` في بداية `main.rs`.

**بدون مفاتيح باينانس:** يعمل وضع قراءة سوق + dry-run؛ لا مزامنة رصيد ولا استعادة مركز موقّعة.

### 3.3 الأوامر

```bash
cargo test                    # 24 اختبار وحدة — يجب أن يمرّ بالكامل قبل الإنتاج
cargo run -- demo             # محلي: حلقة قرار بدون شبكة باينانس
cargo run -- once             # دورة واحدة: WS + REST + قرار (+ أوامر حسب DRY_RUN)
cargo run -- run              # حلقة مستمرة حتى Ctrl+C
```

#### ما الذي يحدث في `once` / `run`؟

1. تهيئة `HoorRunner` و`startup_recovery` (فلاتر، مركز، `tp_stage`).
2. تشغيل خيط WebSocket: `run_market_stream` (`bookTicker` + `aggTrade`).
3. **Warm-up:** انتظار حتى `is_live_ready()` أو فشل `FEED_WARMUP`.
4. دورة/حلقات: بناء `UnifiedMarketState` → `MainLoopPipeline::run_single_cycle`.

#### وضع `demo`

لا WebSocket ولا باينانس؛ يختبر المسار الخطي وKelly فقط.

### 3.4 أخطاء إقلاع شائعة

| الرسالة | السبب المحتمل | الإجراء |
|---------|----------------|---------|
| `FEED_WARMUP: no live bookTicker within …ms` | WS لم يصل خلال المهلة | شبكة، رمز، testnet/mainnet |
| `NO_FEED` | feed غير مربوط | خلل داخلي — لا يحدث في المسار العادي |
| `STREAM_FREEZE` | انقطاع/بطء WS بعد الإقلاع | يُعاد المحاولة في `run`؛ تيليجرام حرج |
| `EMERGENCY_HALT` | 5+ أخطاء exchange متتالية | مراجعة API، حدود، شبكة |

---

## 4. خريطة قنوات تيليجرام

### 4.1 تنبيهات عادية (مبردة)

| القناة | المُشغِّل | التبريد | المحتوى |
|--------|----------|---------|---------|
| **Manual Alert** | `CycleOutcome::ManualAlert` (إجماع 80–84%) | **15 دقيقة / رمز** | رمز، إجماع، تغيّر 24س — **بدون تنفيذ تلقائي** |

الدالة: `exchange/telegram.rs` → `send_manual_alert`.

### 4.2 تنبيهات طوارئ (فورية — بدون تبريد)

| النوع (`Kind`) | المُشغِّل | الدالة |
|----------------|----------|--------|
| `STREAM_FREEZE` | watchdog WS في `resolve_mid` | `notify_stream_freeze` |
| `EXCHANGE_FAILURE_STREAK` | أكثر من **3** أخطاء REST متتالية | `notify_exchange_failure_streak` |
| `EMERGENCY_HALT` | تفعيل `emergency_halt` (عتبة 5) | `notify_emergency_halt` |

القاعدة: `send_critical_alert` — **لا** تستخدم خريطة تبريد Manual.

### 4.3 مخطط تدفق التنبيهات

```text
                    ┌─────────────────────┐
                    │  قرار / Runner      │
                    └──────────┬──────────┘
                               │
           ┌───────────────────┼───────────────────┐
           ▼                   ▼                   ▼
   ManualAlert 80-84    STREAM_FREEZE      Exchange streak / HALT
           │                   │                   │
           ▼                   ▼                   ▼
   send_manual_alert    notify_stream_*      notify_emergency_*
   (cooldown 15m)       (no cooldown)        (no cooldown)
           │                   │                   │
           └───────────────────┴───────────────────┘
                               ▼
                    Telegram sendMessage API
```

**متطلبات:** `HOOR_TELEGRAM_BOT_TOKEN` + `HOOR_TELEGRAM_CHAT_ID`. عند الغياب: سجل `stderr` فقط؛ البوت يستمر.

---

## 5. أمان وامتثال تشغيلي

- **لا تُلتزم** ملفات `.env` أو `hoor_trade_journal.json` (في `.gitignore`).
- استخدم `HOOR_DRY_RUN=true` عند أول تشغيل على mainnet حتى تتأكد من السجلات والتنبيهات.
- دوّر المفاتيح إذا تعرّضت للتسريب.
- راجع `الفهرس.md` عند أي توسعة هيكلية.

---

## 6. قائمة تحقق ما قبل الإنتاج

- [ ] `cargo test` — 24/24
- [ ] `.env` مكتمل (باينانس + تيليجرام حسب الحاجة)
- [ ] `HOOR_DRY_RUN` بالقيمة المقصودة
- [ ] `once` يطبع `FEED_WARMUP: live bookTicker ready`
- [ ] اختبار تنبيه Manual (إن أمكن) وتنبيه حرج (testnet مقصود)
- [ ] مراجعة `RunnerConfig` (TP/SL في `runner/config.rs`)

---

## 7. مراجع داخل المستودع

| الملف | الغرض |
|--------|--------|
| `الفهرس.md` | دستور الهندسة وهيكل الشجرة |
| `README.md` | ملخص المطوّر |
| `.env.example` | قالب المتغيرات |
| `README_HOOR_PRODUCTION.md` | **هذا المستند** — مرجع الإنتاج |

---

*آخر مواءمة: Warm-up إقلاع، قنوات تيليجرام الحرجة/المبردة، CVD حي، Kelly ديناميكي، استعادة tp_stage — 24/24 اختبار.*
