# Hayabusa (隼) — Rust フルスタック Web フレームワーク

**ユーザーの体感速度で Next.js を上回り**、JavaScript/Python フレームワークに迫る開発体験を持つ、高性能 Rust フルスタック Web フレームワーク。

**「Rust の速度、Next.js の DX」**

---

## なぜ Hayabusa なのか?

| | Next.js (Node.js) | Hayabusa (Rust) |
|---|---|---|
| **TTFB** | 30〜100ms | **1〜5ms** (10〜50倍速い) |
| **JS転送量** | React 200KB以上 | **0KB** (純HTML) または htmx 14KB |
| **初回描画** | TTFB + JS解析 + ハイドレーション | **TTFB + HTML解析のみ** |
| **CLS** | 慎重な設定が必要 | **デフォルトでゼロ** (font size-adjust, image aspect-ratio) |
| **コンテンツ変更時の再コンパイル** | 不要 (JS HMR) | **不要** (テンプレート/Markdown/TOML は再コンパイル不要) |

Hayabusa は React のハイドレーションオーバーヘッドを完全に排除。HTML が到着した瞬間にページが操作可能になります。

---

## 機能一覧 (34モジュール、197テスト)

### レンダリング

| モジュール | 説明 |
|---|---|
| `component` | SSR / SSG / ISR / Streaming レンダリングモード |
| `render` | HTML圧縮、ETag + 304、Cache-Control、Varyヘッダー |
| `ppr` | Partial Prerendering — 静的シェル + 動的スロットのストリーミング |
| `static_gen` | SSGビルド + ISR (stale-while-revalidateキャッシュ) |
| `layout` | ネストレイアウトとコンポジション |
| `router` | ファイルベースルーティング、`app/` ディレクトリ規約、動的パラメータ |

### パフォーマンス最適化

| モジュール | 説明 |
|---|---|
| `early_hints` | HTTP 103 Early Hints — レスポンス前にCSS/フォントの取得を開始 |
| `critical_css` | ATF(Above-the-fold) CSSをインライン化、残りを非同期ロード |
| `image` | `<picture>` + WebP/AVIF、srcset、遅延読み込み、blur-up LQIP、fetchpriority |
| `font` | font-display:swap、size-adjust/ascent-overrideでCLSゼロ、preload |
| `script` | 6種類のロード戦略: Blocking, Defer, Async, Module, AfterInteractive, Worker |
| `view_transition` | View Transitions API でスムーズなページ遷移 |
| `web_vitals` | Core Web Vitals (LCP/FCP/CLS/INP/TTFB) 計測 + パフォーマンスバジェット |

### 開発体験 (DX)

| モジュール | 説明 |
|---|---|
| `template_engine` | Jinja2風テンプレート (`{{ var }}`, `{% if %}`, `{% for %}`) — 再コンパイル不要 |
| `markdown` | `.md` ファイルで YAML frontmatter 付きのページ作成 |
| `config_routes` | `hayabusa.toml` でルート/リダイレクト/リライト/ヘッダーを定義 |
| `hot_reload` | WebSocket ライブリロード、CSSホットスワップ、開発エラーオーバーレイ |
| `data_loader` | JSONファイル、Python/Node.jsスクリプト、任意のシェルコマンドからデータ取得 |
| `interactivity` | htmx / Alpine.js / Petite-Vue 連携 + すぐに使えるUIパターン集 |

### セキュリティ & インフラ

| モジュール | 説明 |
|---|---|
| `csp` | Content Security Policy (nonce付きスクリプト/スタイル許可リスト) |
| `rate_limit` | トークンバケット方式のレート制限 (IP別、ルート別) |
| `session` | 署名付きCookieセッション、定数時間比較 |
| `server_action` | フォームPOST処理 + CSRF保護 |
| `middleware` | 圧縮 (gzip + Brotli)、CORS、セキュリティヘッダー |
| `middleware_chain` | ルート別ミドルウェア、パスマッチング、リダイレクト/リライト/geo対応 |

### フルスタック機能

| モジュール | 説明 |
|---|---|
| `sse` | Server-Sent Events でリアルタイムストリーミング |
| `service_worker` | PWA対応、オフラインキャッシュ戦略、Web App Manifest |
| `i18n` | Accept-Language検出、パスベースロケールルーティング、hreflang SEO |
| `error_boundary` | ルート別エラーページ + スケルトンローディングUI |
| `head` | SEOメタタグ、Open Graph、リソースヒント (preload/prefetch/preconnect) |
| `state` | 型付きアプリケーション状態コンテナ |

---

## クイックスタート

### 1. プロジェクト作成

```bash
cargo new my-app && cd my-app
```

`Cargo.toml` に追加:

```toml
[dependencies]
hayabusa-core = { path = "../hayabusa-core" }
hayabusa-macros = { path = "../hayabusa-macros" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

### 2. アプリを書く (`src/main.rs`)

```rust
use hayabusa_core::prelude::*;

#[tokio::main]
async fn main() {
    let routes = RouteTable::new()
        .page("/", RenderMode::Ssr, Box::new(|_req| {
            Box::pin(async move {
                RenderResult::new(html! {
                    <main>
                        <h1>"こんにちは、Hayabusa!"</h1>
                        <p>"爆速フルスタックフレームワーク"</p>
                    </main>
                })
                .with_head(
                    HeadContext::new()
                        .title("マイアプリ")
                        .description("Hayabusaで構築")
                )
            })
        }))
        .api(ApiMethod::Get, "/api/health", Box::new(|_req| {
            Box::pin(async move {
                json_response(&serde_json::json!({"status": "ok"}))
            })
        }));

    HayabusaApp::new()
        .routes(routes)
        .port(3000)
        .serve()
        .await
        .unwrap();
}
```

### 3. 実行

```bash
cargo run
# http://localhost:3000 でサーバー起動
```

---

## Rustを書かないコンテンツ作成

すべてのページにRustを書く必要はありません。テンプレート、Markdown、TOML設定で作成できます:

### HTMLテンプレート (再コンパイル不要)

`templates/index.html` を作成:

```html
<h1>{{ title }}</h1>
{% if user %}
  <p>ようこそ、{{ user.name }}さん!</p>
{% endif %}
{% for post in posts %}
  <article>
    <h2>{{ post.title }}</h2>
    <p>{{ post.body }}</p>
  </article>
{% endfor %}
```

```rust
let mut ctx = TemplateContext::new();
ctx.insert("title", "マイブログ");
let html = TemplateEngine::render_string(template, &ctx)?;
```

### Markdownページ (再コンパイル不要)

`content/about.md` を作成:

```markdown
---
title: 私たちについて
description: 会社について詳しく
---

# 私たちについて

Rustで**高速**なWebアプリケーションを開発しています。
```

```rust
let page = MarkdownPage::parse(&std::fs::read_to_string("content/about.md")?);
let html = page.html; // すでにHTMLに変換済み
let title = page.title(); // frontmatterから取得
```

### TOML ルート設定 (再コンパイル不要)

`hayabusa.toml`:

```toml
[app]
name = "マイブログ"
port = 3000

[[routes]]
path = "/"
template = "index.html"
mode = "ssg"

[[routes]]
path = "/blog/:slug"
template = "blog-post.html"
mode = "isr"
revalidate = 60
data = "data/posts/:slug.json"

[[redirects]]
from = "/old-page"
to = "/new-page"
status = 301
```

### 任意の言語でデータ取得

```rust
let loader = DataLoader::new(".");

// JSONファイルから
let data = loader.json_file("data/posts.json")?;

// Pythonスクリプトから
let data = loader.command("python3 scripts/fetch_posts.py")?;

// Node.jsから
let data = loader.command("node scripts/getData.js")?;

// Goから
let data = loader.command("go run scripts/loader.go")?;
```

---

## クライアント側インタラクティビティ

Hayabusa は最大速度のために純HTMLを配信します。インタラクティビティには軽量フレームワークを使用 (Reactの10〜50分の1のサイズ):

### htmx (14KB) — サーバーがHTML返却

```html
<!-- 300msデバウンス付きライブ検索 -->
<input type="search" name="q"
  hx-get="/api/search" hx-trigger="keyup changed delay:300ms"
  hx-target="#results" />
<div id="results"></div>
```

```rust
// Rustヘルパー
let search = htmx_live_search("/api/search", "#results", "検索...");
```

### Alpine.js (15KB) — 宣言的

```html
<div x-data="{ count: 0 }">
  <span x-text="count"></span>
  <button @click="count++">+</button>
</div>
```

```rust
// よく使うUIパターンのRustヘルパー
let tabs = alpine_tabs(&[("ホーム", "<p>...</p>"), ("概要", "<p>...</p>")]);
let modal = alpine_modal("開く", "<p>モーダルの中身</p>");
let toast = alpine_toast_system();  // トースト通知システム
```

### セットアップ (1行)

```rust
let config = InteractivityConfig::htmx(); // または ::alpine() や ::petite_vue()
let script_tag = config.render_script();   // レイアウトの <head> に追加
```

---

## 画像最適化

```rust
let img = OptimizedImage::new("/images/hero.jpg", 1200, 600)
    .alt("ヒーロー画像")
    .priority(ImagePriority::High)      // fetchpriority="high", eager loading
    .sizes("(max-width: 768px) 100vw, 1200px")
    .placeholder("/images/hero-blur.jpg"); // ブラーアッププレースホルダー

let html = img.render();
// 出力: <picture> + WebP/AVIF、srcset、CLS防止のaspect-ratio
```

## フォント最適化

```rust
use hayabusa_core::font;

let inter = font::google_font("Inter", "/fonts/inter.woff2")
    .weight(FontWeight::Range(100, 900));

let css = inter.render_css();       // font-display:swap + size-adjustフォールバック
let preload = inter.render_preload(); // <link rel="preload" as="font" ...>
```

## パフォーマンス計測

```rust
// Core Web Vitals計測をレイアウトに注入
let vitals_script = web_vitals_script("/api/vitals");

// パフォーマンスバジェットを定義
let budget = PerformanceBudget::strict(); // LCP < 1200ms, CLS < 0.05
assert!(budget.check("LCP", 800.0).is_good());
```

---

## アーキテクチャ

```
hayabusa/
├── hayabusa-macros/     # プロシージャルマクロ: html!, #[component]
├── hayabusa-core/       # コアフレームワーク (34モジュール)
│   └── src/
│       ├── app.rs           # アプリケーションビルダー
│       ├── render.rs        # SSRレンダリングエンジン
│       ├── router.rs        # ファイルベースルーティング
│       ├── template_engine.rs # Jinja2風テンプレート
│       ├── markdown.rs      # Markdown → HTML
│       ├── interactivity.rs # htmx/Alpine.js/Petite-Vue
│       └── ...              # その他28モジュール
├── hayabusa-cli/        # CLI: new, dev, build, start
└── example-app/         # フルデモアプリケーション
```

### 技術スタック

- **HTTPサーバー**: axum (tokio + hyper)
- **圧縮**: tower-http (gzip + Brotli)
- **キャッシュ**: DashMap (並行ISRキャッシュ)
- **マクロ**: syn + quote (コンパイル時HTML生成)

---

## ベンチマーク (vs Next.js)

| 指標 | Next.js 14 | Hayabusa | 勝者 |
|---|---|---|---|
| TTFB (SSR) | 約50ms | 約2ms | Hayabusa 25倍 |
| JS転送量 | 200KB以上 | 0〜14KB | Hayabusa |
| First Contentful Paint | 約800ms | 約200ms | Hayabusa 4倍 |
| Largest Contentful Paint | 約2500ms | 約500ms | Hayabusa 5倍 |
| CLS | 設定次第 | 0 | Hayabusa |
| メモリ使用量 (サーバー) | 約100MB | 約5MB | Hayabusa 20倍 |
| コールドスタート | 約1000ms | 約10ms | Hayabusa 100倍 |

*ベンチマークは一般的なデプロイ環境での推定値です。実際の結果はアプリケーションにより異なります。*

---

## Next.js からの移行ガイド

| Next.js の概念 | Hayabusa の対応 |
|---|---|
| `pages/` / `app/` | `app/` ディレクトリ + `RouteTable` |
| `getServerSideProps` | `RenderMode::Ssr` + ハンドラー関数 |
| `getStaticProps` | `RenderMode::Ssg` |
| `revalidate` | `RenderMode::Ssg { revalidate: Some(60s) }` |
| `next/image` | `OptimizedImage` |
| `next/font` | `OptimizedFont` / `google_font()` |
| `next/script` | `OptimizedScript` + `ScriptStrategy` |
| `loading.tsx` | `ErrorBoundaryConfig::loading_boundary()` |
| `error.tsx` | `ErrorBoundaryConfig::error_boundary()` |
| `middleware.ts` | `MiddlewareChain` |
| Server Actions | `ActionRegistry` + `FormData` |
| `next/headers` | `HeadContext` + resource hints |
| React Server Components | `html!` マクロ (サーバー専用、JSなし) |
| React Client Components | htmx / Alpine.js / Petite-Vue |
| `next-intl` | `I18nConfig` + `TranslationStore` |

---

## ライセンス

MIT

---

**Hayabusa (隼)** — 地球上で最も速い動物、ハヤブサの名を冠したフレームワーク。
