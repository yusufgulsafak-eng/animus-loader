# Shadows Loader integration

The supplied ORIJINAL package needs a new `INSTALL_SHADOWS` action. Loader 0.1.5
converts its 82 localization records and runs the pinned append-only injector on
five fixed FORGE paths. The payload's manifest hash is pinned in the native
adapter, PHP validator and schema; it cannot select a script or arbitrary tool.
The PowerShell implementation is embedded in Loader. Package scripts are ignored.

Before the injector runs, Loader registers the transaction and the embedded
installer flushes each original index snapshot and its undo journal. It records
the complete original hash and the original payload body hash. Killed helper
processes are contained in a Windows job. Uninstall preflights all snapshots and
all original bodies before restoring any index and truncating appended data.
It then verifies the complete original hashes. Installed journals and snapshots
are also copied into Loader's primary and independent recovery backups.

An incomplete transaction remains recoverable from its game journal. Missing or
corrupted recovery material stops restoration; `force` does not bypass the
Shadows checks. A child error explicitly confirmed before all game writes can
close an unused transaction. Any other missing pending journal retains the
transaction for investigation. Completed game backup directories become history.

## Package and server configuration

Run `python tools/build_shadows_payload.py SOURCE.zip OUTPUT.zip` using the user's
original `ANIMUS_AC_SHADOWS.zip`. It checks all 1314 source file hashes and creates
a deterministic payload containing the original data, manifest, instructions and
`LOADER_PROFILI.json`. It omits the manual BAT and PS1 entry points. Helper EXEs and
the DLL stay byte-identical. Translation data/binaries are not committed here.

Apply `web/database/030_shadows_loader_action.sql` with the server's usual migration
process and deploy the updated validator/admin code. The SQL preserves existing
enum values and does not publish or modify a game. No live migration has been
performed as part of this source change.

Configure the existing original-game entry with executable and process
`ACShadows.exe`, minimum Loader `0.1.5`, and these required files:

- `DataPC_boot.forge`
- `DataPC_boot_patch_01.forge`
- `DataPC_boot_patch_02.forge`
- `dlc_20/DataPC_boot_20_dlc.forge`
- `dlc_20/DataPC_boot_20_dlc_patch_01.forge`

Use the single action from `LOADER_PROFILI.json`. Upload the resulting ZIP to the
chosen MediaFire location, then configure that archive's actual URL, byte size and
SHA-256 in the existing external-archive form. The existing download flow handles
MediaFire. No URL, database IDs or live patch version are invented by this change.

## Verification

- Native modules: `cargo test --locked --manifest-path tests/fat-dat-harness/Cargo.toml`.
- Actual embedded restore script: `powershell -File tests/shadows-recovery.ps1`.
  It covers normal uninstall, a partly applied install, changed original body and
  corrupted snapshot, plus resource-ID preflight and atomic journal updates.
- Server contracts: `php tests/php/run.php`.
- Frontend: `npm test` and `npm run build` from `loader`.

The dedicated Shadows workflow runs these checks on Windows PowerShell 5.1 and
builds the actual Tauri release artifacts. Local Linux module/fixture checks do
not establish Windows binary or in-game compatibility. No real-game test is
claimed. Only the supplied ORIJINAL package is pinned; no cracked-edition profile
is included.
