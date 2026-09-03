# Network tricks delivery checklist

This branch is stacked on upstream PR #21. Tailscale is treated as a private routed network,
never as a filesystem protocol: Flea discovers peers through Tailscale, then reaches their files
through SFTP, SMB, NFS, or WebDAV.

## Completion rules

- [x] Do not check an implementation item until its named automated test exists and passes.
- [x] For every command-driven feature, cover failure, empty, malformed, unavailable, and timeout cases where the command can produce them.
- [x] Keep every Rust/QML file under 400 lines and every JavaScript file under 300 lines.
- [x] Run the complete runnable test matrix once, fix every product regression, then run it again.
- [x] Record both clean verification passes below; one pass cannot satisfy both boxes.

## 1. Tailscale peer discovery

- [x] Parse all usable non-relay peers, including offline peers, without conflating Taildrop eligibility.
- [x] Add discovered peer rows to NETWORK with online/offline state and stable identity.
- [x] Open a peer through a chosen file protocol, defaulting to SFTP rather than inventing a Tailscale filesystem.
- [x] Tests: `tests/js/tailnet.js`, `tests/js/places.js`, and `tests/network-services.sh`.

## 2. Zero-configuration LAN discovery

- [x] Parse bounded `avahi-browse -artp` output for SSH, SMB, NFS, and WebDAV services.
- [x] Deduplicate discovered services against bookmarks and live mounts.
- [x] Keep discovery optional: absent, malformed, failed, or timed-out Avahi must leave manual mounts working.
- [x] Tests: `tests/js/discovery.js`, `tests/js/places.js`, and `tests/network-services.sh`.

## 3. Quick Connect

- [x] Parse `user@host:/path`, `host:/path`, `host/share`, full URIs, IPv4, and IPv6 safely.
- [x] Show the normalized URI before saving and never guess SMB when an explicit scheme exists.
- [x] Wire Quick Connect into the existing network dialog without weakening the structured form.
- [x] Tests: `tests/js/quickconnect.js`, protocol tests, and the updated `tests/ui.sh network` case.

## 4. Favorites and recent connections

- [x] Keep GTK network bookmarks as favorites and add a bounded, deduplicated recent-success list.
- [x] A failed attempt must not become recent; a successful open must move one connection to the front.
- [x] Persist recents atomically without storing passwords, tokens, or embedded URI secrets.
- [x] Tests: `tests/js/recents.js` and FileView persistence in `tests/network-services.sh`.

## 5. Connection health

- [x] Represent discovered/bookmarked peers as online, offline, unknown, connecting, mounted, or failed.
- [x] Preserve the last good rail during a timed-out poll and expose a precise status sentence.
- [x] Bound every discovery/health process; no dead host may freeze the rail.
- [x] Tests: health-state units and wedged Quickshell processes in `tests/network-services.sh`.

## 6. Remote context actions

- [x] Add Copy address, Open terminal over SSH, Send with Taildrop when eligible, Reconnect, and Unmount.
- [x] Gate actions by capability and state; a row must never offer an action that cannot run.
- [x] Keep argv boundaries intact for hostile labels, hosts, users, ports, and paths.
- [x] Tests: `tests/js/remote.js`, `tests/js/mounts.js`, and stubbed commands in `tests/network-services.sh`.

## 7. Wake-on-LAN

- [x] Validate and normalize MAC addresses before any packet is sent.
- [x] Send the canonical magic packet without a shell and report success/failure precisely.
- [x] Offer Wake only to saved LAN entries carrying a valid MAC; never to Tailscale-only peers.
- [x] Tests: Rust packet/validation tests, `tests/modes.sh`, and the non-network stub in `tests/network-services.sh`.

## 8. Remote-to-remote transfers

- [x] Recognize when source and destination are mounted remote paths while reusing the existing copy/move backend.
- [x] Make the transfer explicit in status/progress text and preserve cancel/partial-file guarantees.
- [x] Prove copy and move between two synthetic mounted roots; the shared backend's failure and cancellation tests remain authoritative.
- [x] Tests: `tests/ops.sh`, Rust copy/move failure/cancel tests, and `tests/js/remote.js`/`tests/js/ops.js`.

## 9. Tailscale troubleshooting

- [x] Distinguish CLI missing, daemon stopped, logged out, no peers, and ready.
- [x] Show recovery guidance without executing privileged or authentication-changing commands.
- [x] Keep ordinary LAN/GIO connections available in every Tailscale failure state.
- [x] Tests: `tests/js/tailnet.js` plus stubbed CLI process coverage for every state.

## Verification pass 1

- [x] `cargo clean && cargo build && cargo build --release && cargo test && cargo test --release`
- [x] `./tests/run-all.sh` — 11 suites, 0 failed.
- [ ] `tests/ui.sh network networkedit sharebrowser unmount taildrop` — blocked locally because `omarchy-drive` is not installed; the real GUI loads and the process-level network harness passes.
- [x] `./tools/flea-qmllint-gate`
- [x] `./tools/flea-file-budget`
- [x] `git diff --check`
- [x] Real GUI instantiation: `flea --gui /tmp` reached `Configuration Loaded` and stayed healthy until the deliberate timeout.

## Bug-fix interval

- [x] Bracketed IPv6 was rejected by the inherited protocol parser; covered in `tests/js/protocols.js`.
- [x] A stale Wake profile survived URI edits or clearing the MAC; covered in `tests/js/recents.js`.
- [x] A mounted discovered row offered Unmount but the service rejected its kind; the guard now follows capability.
- [x] Clipboard and Wake reported success before their child exited; success/failure is covered in `tests/network-services.sh`.
- [x] Recent rows shadowed fresh discovery health, Taildrop, or Wake metadata; covered in `tests/js/places.js`.
- [x] Delayed unmount cleared its URI before health cleanup; the pending identity now survives through `onExited`.
- [x] Bare-server share listing could hang forever; a wedged `gio list` is covered in `tests/network-services.sh`.
- [x] Explicit/shorthand Quick Connect could persist query tokens or fragments; covered in `tests/js/quickconnect.js`.
- [x] IPv6 copy-address text lost one bracket and custom SSH ports were unusable; covered in place/action tests.
- [x] The repository fixture/dependency failures were environmental, not product regressions: tests passed with namespace access, optional `7zip`, and the documented guarded media fixture.

## Verification pass 2

- [x] Repeat the clean four-profile Rust warning gate — 341 tests per profile, zero warnings.
- [x] Repeat `./tests/run-all.sh` from fresh fixtures — 11 suites, 0 failed.
- [x] Repeat the process-level network cases from fresh stubs/fixtures; repeat real GUI loading.

## Pointer scrolling follow-up

- [ ] Reproduce the slow default Qt wheel behavior and record the expected accelerated distance.
- [ ] Use one shared vertical-scroll policy for list, grid, Miller columns, sidebar, text preview, and PDF preview.
- [ ] Preserve smooth touchpad pixels, discrete wheel-line preferences, hard bounds, pointer taps, and drag ownership.
- [ ] Let wheel input propagate at a scroll boundary instead of trapping nested views.
- [ ] Add deterministic unit coverage for pixel deltas, wheel notches, direction, multiplier, empty input, and both bounds.
- [ ] Add structural coverage proving every shipped vertical Flickable uses the shared policy exactly once.
- [ ] Run the complete runnable test matrix twice after the scrolling fix, plus QML lint, file budgets, and diff checks.
- [ ] Run pointer-driven scrolling against the real window, or record the exact missing driver and the substitute live check.
- [ ] Split the finished work into reviewable commits/PRs, push every branch, and record the dependency order.
- [x] Repeat QML lint, file-budget, and whitespace gates.
- [ ] Repeat pointer-driven `tests/ui.sh` cases when `omarchy-drive` is available.
