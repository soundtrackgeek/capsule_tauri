## Development Workflow

**IMPORTANT**: After any code change, bug fix, or feature addition/removal, you MUST complete all of these steps:

1. **Update README.md** if the change affects:
   - Usage examples or commands
   - Installation instructions
   - Configuration options
   - Available features

2. **Update CHANGELOG.md**:
   - Add new version number following semantic versioning (MAJOR.MINOR.PATCH)
   - Add entry under appropriate category (Added, Changed, Fixed, Removed)
   - Include date in format YYYY-MM-DD

3. **Clean Rust build artifacts**:
   - After finishing verification/build/test commands and before the final handoff,
     run `cargo clean` from the `src-tauri` directory:
     `cargo clean`
   - Do this for every repo work session, including docs-only work, unless the
     user explicitly asks to preserve build artifacts.
   - Report if the cleanup command cannot be completed.

4. **Commit and push changes**:
   - Use `git add` to stage all modified files (README.md, CHANGELOG.md, and code files)
   - Create descriptive commit message following existing style
   - Push to remote repository with `git push`
   - If the change bumps the app version for a release, ensure `package.json`,
     `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`,
     and `CHANGELOG.md` all use the same version
   - After pushing a release version commit, create and push the matching Git tag
     in `vMAJOR.MINOR.PATCH` format, for example `git tag v0.8.1` followed by
     `git push origin v0.8.1`
   - Pushing the version tag triggers the `Release Windows Installer` GitHub
     Actions workflow, which builds the Windows Tauri bundles and uploads the
     generated NSIS setup executable and MSI to the matching GitHub Release
   - Do not create a release tag for docs-only changes or non-release work
