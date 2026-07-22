# Contributing to Toolpulse

Thanks for your interest in contributing! 🎉 This guide covers everything you need to get started.

## Quick start

```bash
git clone https://github.com/<owner>/toolpulse.git
cd toolpulse
npm install
npm run tauri dev
```

**Prerequisites:** Node.js 20+, Rust stable ([rustup](https://rustup.rs/)), and the [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/).

## Project structure

```
src-tauri/src/     Rust backend (scanning, commands, tray, scheduler)
src/               React frontend (components, hooks, types)
src/components/    UI components
src/hooks/         React hooks (scan state, terminal runs)
src/lib/           API wrappers, icon helpers, tree builder
```

## How to add a new tool

Append a `ToolDefinition` to `builtin_tools()` in `src-tauri/src/tools/registry.rs`. Install/uninstall/upgrade commands are attached automatically via `apply_default_actions`. No other code changes needed.

## How to add a new ecosystem to the project scanner

1. Add the manifest filename to `SIGNATURES` in `src-tauri/src/tools/projects.rs`.
2. Add a parser in `parse_deps_detail`.
3. Add the ecosystem to `ProjectEcosystem` in `types.rs` and `ECOSYSTEM_META` in `src/types.ts`.

## Code style

- **Rust:** `cargo fmt` + `cargo clippy` before committing.
- **TypeScript:** `npx tsc --noEmit` must pass. Match the existing style (functional components, hooks, no class components).
- Comments: explain *why*, not *what*. Every public function has a doc comment.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add support for Deno projects
fix: tree grouping when projects are in /Volumes
docs: update README with Windows instructions
refactor: lift scan state to App level
```

## Pull requests

1. Fork the repo and create a branch: `git checkout -b feat/my-feature`
2. Make your changes. Ensure `cargo check` and `npx tsc --noEmit` pass.
3. Commit with a conventional commit message.
4. Push and open a PR. Describe what changed and why.

## Reporting bugs

Use the in-app "Report on GitHub" button (it pre-fills system info), or [open an issue](https://github.com/<owner>/toolpulse/issues/new/choose) directly.

## License

By contributing, you agree that your contributions are licensed under the MIT License.
