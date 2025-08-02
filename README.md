# Mboî Tatá 🔥🐍

**Mboî Tatá** is a modular tool for intercepting, analyzing, and extracting `.js`, `.map`, and sensitive data from HTTP(S) traffic. Inspired by asynchronous pipelines and a stage-based architecture, it enables flexible, secure, and extensible operations.

---

## 🔧 Features

* HTTP/S MITM Proxy with self-signed TLS support.
* Captures JavaScript and `.map` files for source map analysis.
* Asynchronous dispatcher with inflight event control and graceful shutdown.
* Optional automated browsing with headless Chrome.
* Scope control via allowlist or input files.
* Modular architecture based on stages.

---

## ⚖️ Architecture

```text
InterceptedResponse
     ↓
 [FilterStage] → [JsSaveStage] → [MapStage] → [ScanStage]
     ↓
 Dispatcher coordinates events between stages.
```

---

## 🚀 How to Use

### Installation

```bash
cargo build --release
```

### Basic Execution

```bash
cargo run -- --port 8085 --urls urls.txt --allowlist example.com
```

### Execution with stdin

```bash
cat urls.txt | cargo run -- --urls -
```

### Available Parameters

```bash
--urls           List of URLs for the browser (can use '-')
--allowlist      Allowed domains for interception
--port           Proxy port (default: 8085)
--output         Output folder (default: ./output)
--certs          TLS certificate folder (default: ./certs)
```

---

## ⚙️ Dispatcher and Graceful Shutdown

* The `Dispatcher` is the core component that:

  * Coordinates events between stages.
  * Tracks how many events are being processed (`inflight`).
  * Uses `Notify` to await completion before shutdown.
  * Signals shutdown using a `broadcast` channel.

---

## 🔍 Internal Details

* URLs can be passed via `--urls` or `stdin`, with automatic domain parsing into the allowlist.
* If no filter is provided, the proxy intercepts everything.
* The stage system can be extended by implementing the `Stage` trait.

---

## 📁 Example Configuration Code

```rust
let (config, allowlist) = config::load();
let (dispatcher, stage_handle) = StageRegistry::default()
    .register(StageId::Filter, Box::new(FilterStage))
    .register(StageId::JsSave, Box::new(JsSaveStage))
    .build();
```

---

## 📚 Etymology

**Mboî Tatá** comes from Tupi, meaning "fire serpent" — a mythological creature that burns the trails it passes. This tool follows that spirit: silent, powerful, and destroyer of hidden vulnerabilities.

---

## 🚫 Legal Notice

This tool is intended for educational and Red Team use in controlled environments. Misuse may be illegal.

---

## ✨ Contributing

Suggestions, feedback, and contributions are warmly welcome! Feel free to open PRs or issues with ideas, questions, or improvements.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
