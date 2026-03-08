---
name: pr
description: "Create PR Workflow: show diff summary, run cargo clippy + cargo test, wait for approval, then create PR."
user-invocable: true
---

# Create PR Workflow

1. Show me the diff summary and proposed PR title/description
2. Wait for my approval on the title/description
3. Run `cargo clippy` (zero warnings) and `cargo test` (all pass)
4. Create PR with clean, well-formatted description
