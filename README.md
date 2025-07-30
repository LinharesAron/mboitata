# 🔥 Mboîtatá - The Blazing Sentinel of the Web

**Mboîtatá** is a handcrafted proxy written in 🦀 Rust, forged with the flames of Brazilian folklore. Its purpose burns bright: to intercept and collect sensitive and strategic web artifacts like `.js`, `.map`, backend URLs, keys, and other hidden treasures from HTTP and HTTPS traffic.

> ⚠️ This is an early-stage project, built as a learning experience. Some flames still flicker, but it already packs enough heat to scorch some bugs! 🌶️

---

## 🔍 Purpose

To assist in collecting valuable data during web application reconnaissance. Mboîtatá intercepts both HTTP and HTTPS requests and responses, saving useful files that often reveal:

* JavaScript source code
* Debug `.map` files
* Internal URLs and endpoints
* Secrets, API keys, tokens
* Internal configurations (`.env`, `.conf`, etc.)

---

## 🔧 How to Use

```bash
cargo run
```

The tool is still in its early stage, but it already supports:

* Starting a basic HTTP proxy
* Saving files based on their extension
* Preparing the structure for more advanced analysis stages

---

## 🔥 Inspiration

> *"I am the fire that watches in the dark. No secret escapes my burning gaze."*

Inspired by Boitatá — the fiery serpent guardian of the forests — this project brings the strength and mystique of Brazilian folklore into the world of offensive cybersecurity.

---

## ⚠️ Disclaimer

This project is intended for educational use and authorized environments only. Unauthorized use against systems without permission is illegal and unethical.

---

## ✨ Contributing

Suggestions, feedback, and contributions are warmly welcome! Feel free to open PRs or issues with ideas, questions, or improvements.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
