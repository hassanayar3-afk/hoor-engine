# تحميل وتشغيل سكربت إعداد HOOR (فوري)

## الملف الجاهز

| الملف | المسار المحلي |
|--------|----------------|
| **سكربت الإعداد الكامل** | `C:\Users\acer\hur\scripts\HOOR_SETUP_COMPLETE.ps1` |
| دليل الإنتاج | `README_HOOR_PRODUCTION.md` |
| قالب بيئة آمن (بدون أسرار) | `.env.example` |

بعد رفع المستودع على GitHub، يمكن تنزيل السكربت من:

`https://github.com/hassanayar3-afk/hoor-engine/blob/main/scripts/HOOR_SETUP_COMPLETE.ps1`

(زر **Raw** → حفظ باسم `HOOR_SETUP_COMPLETE.ps1`)

## تشغيل فوري على جهازك (Windows)

```powershell
cd C:\Users\acer\hur
powershell -ExecutionPolicy Bypass -File .\scripts\HOOR_SETUP_COMPLETE.ps1
```

## ماذا يفعل السكربت؟

1. يتحقق من `cargo`
2. ينشئ `.env` **محلياً** (يطلب منك API و Telegram — **لا يكتبها في GitHub**)
3. `cargo test` → `cargo build --release`
4. `cargo run -- verify` → `cargo run -- once`
5. اختياري: `git push` للكود فقط مع فحص أن `.env` غير مُتتبَّع

## أمان إلزامي

- **لا ترفع `.env` أبداً** إلى `hassanayar3-afk/hoor-engine`
- مفاتيح **Testnet** من: https://testnet.binancefuture.com
- مفاتيح **Demo** من باينانس مع `HOOR_MODE=demo`
- تحقق: `cargo run -- verify`

## أوامر يدوية سريعة

```powershell
cd C:\Users\acer\hur
cargo test
cargo run -- verify
cargo run -- once
cargo run -- run
```

## رفع GitHub يدوياً (بدون أسرار)

```powershell
cd C:\Users\acer\hur
git status
git check-ignore -v .env
git add .
git commit -m "your message"
git push -u origin main
```

المستودع: https://github.com/hassanayar3-afk/hoor-engine
