# حُر — HOOR Engine v1.0

محرك تداول **أحادي السيادة** بلغة Rust: مسار قرار **خطي واحد**، صفقة **واحدة** قفل ذري، درع أوزان محلي، Kelly جزئي + ATR، وفحص عمق السيولة.

## هيكل المجلدات (كل ملف = اختصاص واحد)

| المسار | الاختصاص |
|--------|----------|
| `src/config.rs` | ثوابت الدستور (500$، سقف 4000 وزن، عتبة إجماع 85) |
| `src/types/market_state.rs` | `UnifiedMarketState` — لقطة السوق `repr(C)` |
| `src/guards/chrono.rs` | `ChronoGuard` — وقت البورصة الآمن |
| `src/guards/rate_limit.rs` | `LocalWeightBucket` — منع 429/418 |
| `src/validation/depth.rs` | `OrderBookDepthValidator` + `OrderSide` |
| `src/engine/kelly.rs` | حساب نسبة المركز (Kelly 30% + ATR) |
| `src/engine/state.rs` | حالة المحرك الذرية |
| `src/engine/decision.rs` | المستويات 0→3 للقرار الخطي |
| `src/pipeline/` | ربط الحلقة: مدخل دورة + `MainLoopPipeline` |
| `src/exchange/` | باينانس REST — قراءة/أوامر بلا قرار |
| `src/runner/` | دورة دخول/خروج + Kelly + مزامنة رصيد |
| `src/engine/exit.rs` | TP/SL ومنطق الإغلاق |
| `src/main.rs` | `demo` / `once` / `run` |

## المسار العصبي الخطي

```
ماكرو + أوزان → صفقة مفتوحة؟ → إجماع ≥ 85 → عمق 5× → ATTACK + قفل ذري
```

## التشغيل

```bash
cargo test
cargo run -- demo    # بدون باينانس
cargo run -- once    # دورة واحدة مع Futures REST
cargo run -- run     # حلقة مستمرة (Ctrl+C للإيقاف)
```

انسخ `.env.example` إلى `.env`. الافتراضي **dry-run** (لا أوامر حقيقية). التفاصيل في `الفهرس.md`.
