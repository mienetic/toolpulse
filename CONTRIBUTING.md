# Contributing to Toolpulse

Thanks for your interest in contributing! 🎉 This guide covers everything you need to get started.

## Quick start

```bash
git clone https://github.com/mienetic/toolpulse.git
cd toolpulse
npm ci
npm run tauri dev
```

**Prerequisites:** Node.js 20+ (including npm), Rust stable ([rustup](https://rustup.rs/)), and the [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/) for your operating system.

## Project structure

```
src-tauri/src/     Rust backend (scanning, commands, tray, scheduler)
src/               React frontend (components, hooks, types)
src/components/    UI components
src/hooks/         React hooks (scan state, terminal runs)
src/lib/           API wrappers, icon helpers, tree builder
```

## How to add a new tool

Append a `ToolDefinition` to `builtin_tools()` in `src-tauri/src/tools/registry.rs`. Install/uninstall/upgrade commands are attached automatically via `apply_default_actions`. No other code changes are needed.

## How to add a new ecosystem to the project scanner

1. Add the manifest filename to `SIGNATURES` in `src-tauri/src/tools/projects.rs`.
2. Add a parser in `parse_deps_detail`.
3. Add the ecosystem to `ProjectEcosystem` in `types.rs` and `ECOSYSTEM_META` in `src/types.ts`.

## Code style

- **Rust:** Run `cargo fmt --check --manifest-path src-tauri/Cargo.toml` and `cargo clippy --manifest-path src-tauri/Cargo.toml` before committing.
- **TypeScript:** Run `npx tsc --noEmit`. Match the existing style (functional components and hooks).
- **Comments:** Explain *why*, not *what*, and document public APIs where their purpose or constraints are not obvious.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add support for Deno projects
fix: tree grouping when projects are in /Volumes
docs: update README with Windows instructions
refactor: lift scan state to App level
```

## Pull requests

1. Fork the repository and create a branch: `git checkout -b feat/my-feature`.
2. Make your changes.
3. Run the same core checks as CI:

   ```bash
   npx tsc --noEmit
   cargo check --manifest-path src-tauri/Cargo.toml
   ```

4. Commit with a conventional commit message.
5. Push the branch and open a PR that explains what changed and why.

## Reporting bugs

Use the in-app "Report on GitHub" button (it pre-fills system info), or [open an issue](https://github.com/mienetic/toolpulse/issues/new/choose) directly.

## License

By contributing, you agree that your contributions are licensed under the MIT License.
