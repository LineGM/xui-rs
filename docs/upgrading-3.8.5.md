# Upgrade to 3x-ui 3.8.5

xui-rs 2.0.0 targets the stable [3x-ui v3.8.5 tag](https://github.com/MHSanaei/3x-ui/releases/tag/v3.8.5),
commit `7ef22f94c950ff09f0870e2295fa65ad5968742c`. The review covers all
199 commits since v3.7.0: 160 in v3.8.0 and 39 more in v3.8.5. The inventory
below lists every commit, including maintenance and UI-only changes. Contract
changes were checked against tagged Go controllers, models, service response
types, subscription handlers, and broadcaster call sites, as well as OpenAPI.

## Rust migration from 1.x

This is a major SDK release because public struct fields changed and fractional
balancer weights cannot implement `Eq`. Constructors and endpoint names remain
available, with these source changes:

- Change the dependency requirement to `xui-rs = "2"`.
- `ClientConfig.keep_alive` is now `Option<i32>`: `None` preserves the stored
  value, `Some(0)` explicitly disables keepalive, and `Some(n)` sets seconds.
  `ClientRecord::to_config()` includes the fetched value explicitly.
- Struct literals for extended models need their new fields. Prefer existing
  constructors or `..Default::default()` where supported.
- `SubscriptionBalancer` and `SubscriptionBalancerInput` retain `PartialEq`
  but no longer implement `Eq`; `member_weights` is `BTreeMap<i64, f64>`.

The public API snapshot records these intentional changes. The Rust MSRV remains
unchanged. The minimum rustls version is raised to 0.23.45 to address
[RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html),
which the release dependency check found in the existing lockfile.

## Changes that affect SDK callers

| Upstream contract | SDK update / behavior |
| --- | --- |
| TUIC v5 | `InboundProtocol::Tuic`, remote protocol support, typed `TuicServerSettings` in inbound options; arbitrary inbound settings still use JSON. |
| 42 additional persisted settings | All 147 tagged fields round-trip, including Discord, Happ, REALITY candidates, profile mode, info/status templates, JSON DNS/routing, and inclusive expiry presentation. |
| Discord notifications | `DiscordSettings`, token-presence and explicit token-clear controls, `settings().test_discord()`. |
| Happ Crypt5 links | `clients().happ_link(id)` returns a redacted `HappLink`. The server generates it locally. |
| HWID status / fingerprint | `SubscriptionClient::hwid_status` and HEAD metadata; `ClientHwidDevice::fingerprint`. Reading slot status does not register a device. |
| Subscription aliases | GET/HEAD helpers for `/mihomo/` and `/clash-legacy/`; aliases can be unavailable when a configured format owns their path. |
| Happ response headers | `SubscriptionMetadata::happ` preserves application management headers with redacted debug output. |
| Balancer weights | JSON-encoded `memberWeights` in form submissions, alongside repeated `inboundIds`; fractional weights are preserved. |
| Bulk client adjustments | Optional `limit_hwid`, including explicit zero; `ad_tag`, where empty preserves and `none` clears. |
| Tunnel keepalive | Omitted and zero are distinct in writable configuration. |
| Outbound subscription User-Agent | Stored input/output field plus preview method accepting a custom user agent. |
| Geodata presets | Typed `GeodataSource` values in the Xray settings snapshot. |
| REALITY scan | Certificate-chain byte count and previously omitted private-target/chain-valid fields; corrected lowercase wire names with legacy aliases. |
| Nullable collections | Empty inbound links and panel/Xray logs normalize null to empty vectors. |
| WebSocket documentation | Nine real event descriptions checked against the ten source constants; no runtime handshake or envelope change. |

Other relevant upstream behavior is implemented by the server and needs no
new client-side transformation:

- Invalid/disabled Bearer tokens now return 401. Missing credentials and wrong
  base paths can still return 404; the SDK already classifies 401 correctly.
- Negative inbound subscription sort indices are preserved; zero/omitted
  creation values normalize to one. The dedicated setter rejects zero.
- Profile pages default to `subProfileMode=none`; `profile-web-page-url` can be
  absent. Explicit `builtin` and `custom` modes opt in. Existing custom URLs
  are migrated by the server.
- Manual client disable/delete may restart Xray when the restart-on-disable
  setting is enabled. Port collisions and unusable configs are rejected;
  an already running core can keep serving with a non-empty status error.
- Node operations fan out concurrently, and partial failures can follow
  partial application. Non-idempotent mutations must not be blindly retried.
- Fresh installations randomize subscription paths. Configure the public
  subscription client from panel settings rather than assuming `/sub/`.
- NordVPN responses remain open-ended sensitive JSON. Multi-server results,
  new AmneziaWG outbounds, Xray migrations, subscription rendering, node sync,
  Telegram changes, and installer fixes are handled upstream without SDK wire
  translation.

## How LineGM's contribution entered upstream

[PR #6409](https://github.com/MHSanaei/3x-ui/pull/6409) was merged on
September 4 as [`ed6bc1d8`](https://github.com/MHSanaei/3x-ui/commit/ed6bc1d8).
It aligned generated OpenAPI with actual runtime contracts, including typed
responses, authentication, request encodings, and WebSocket envelopes/events.
[PR #6430](https://github.com/MHSanaei/3x-ui/pull/6430) followed on September 8
as [`b8597314`](https://github.com/MHSanaei/3x-ui/commit/b8597314), documenting
nullable collection responses, including Xray logs. Both are credited to
LineGM in the [v3.8.0 release notes](https://github.com/MHSanaei/3x-ui/releases/tag/v3.8.0).
The contribution was API contracts, generation, documentation and supporting
response types/tests; it was not a port of the Rust SDK into the Go server.

Two maintainer commits on September 15 explain the later WebSocket changes:

- [`7fc86f87`](https://github.com/MHSanaei/3x-ui/commit/7fc86f87) preserves
  unchanged frontend inbound rows and online sets across WebSocket pushes.
  This optimizes React rendering without changing server payloads.
- [`01ce2bce`](https://github.com/MHSanaei/3x-ui/commit/01ce2bce) moves the
  WebSocket event cards and Panel API into separate tabs, with per-section
  Swagger tabs. The event cards continue to use the corrected event examples.

Both commits are **after v3.8.0 but included in v3.8.5**. They do not enable
Bearer authentication on `/ws`. The hub/controller and envelope remain
unchanged between v3.7.0 and v3.8.5.

## After v3.8.5

Main was inspected separately through September 28 (GitHub head
`823db059660dde3123d79fc67928da2e29696903`). Its changes are not included in
the pinned SDK contract. Notable future changes include host cipher-suite
overrides, disabled-limit HWID listing, `excludeFromSub`, portable traffic
counters, weekly renewal/schedule previews, additional Happ/INCY controls,
external subscription User-Agent settings, and inbound/client lifecycle race
fixes. No post-v3.8.5 WebSocket protocol change was found in that range.

## Regression checks and CI

Every push and pull request runs the mock/contract suite on Linux, macOS, and
Windows; strict Clippy, documentation checks, Rust 1.88 MSRV, the pinned-nightly
public API snapshot, package verification, and dependency policy checks remain
mandatory workflow jobs. Pull requests also run the SemVer comparison.

The shared live workflow now runs against the digest-pinned 3.8.5 container on
every push/PR and before release. It checks cookie authentication, a real decoded
WebSocket status event, disabled-token HTTP 401 responses, the runtime route
inventory, inbound CRUD and negative sorting, fractional balancer weights, and
geodata presets. Mutation guards and resource cleanup remain enabled.

The line-coverage floor is 95%. CI retains LCOV and live-test logs as artifacts.
New regression cases explicitly cover omitted versus zero keepalive/HWID values,
nullable collections and weights, failed mutations, malformed HWID/Happ replies,
subscription identifier redaction, and all 147 persisted settings.

## Complete release commit inventory

The area column identifies where the change belongs; a server-side fix may
change operation outcomes without changing HTTP request/response types.

### v3.7.0 → v3.8.0

| Commit | Change | Area |
| --- | --- | --- |
| [`f9cfd87c`](https://github.com/MHSanaei/3x-ui/commit/f9cfd87cb235e150a18b5a327e63db6ff0b13d73) | feat(nord): support multi-server NordLynx outbounds (#6311) | Server runtime |
| [`7100fbcd`](https://github.com/MHSanaei/3x-ui/commit/7100fbcd086954d4a714492f570f3498321b9f20) | feat(sub): leastLoad member weights for subscription balancers (#6304) | API / server / subscription contract |
| [`1bf078c5`](https://github.com/MHSanaei/3x-ui/commit/1bf078c51e2a1089fd76982a46953562ef98cd5a) | feat(routing): add panel-only comment field to routing rules (#6361) | Server runtime |
| [`71607e38`](https://github.com/MHSanaei/3x-ui/commit/71607e386152dc152d91bd723692f26eb9f8c962) | fix: Prevent node snapshots from resurrecting bulk-deleted clients (#6382) | Server runtime |
| [`b8121613`](https://github.com/MHSanaei/3x-ui/commit/b81216135d103acc08d158936c5f8e5ff65228f7) | fix(clients): sync auto-renewal across inbounds (#6339) | Server runtime |
| [`f6445304`](https://github.com/MHSanaei/3x-ui/commit/f64453041a6d0950dee684745fcaced104a0879a) | fix: preserve per-inbound WireGuard peer addresses (#6344) | Server runtime |
| [`8abe87b6`](https://github.com/MHSanaei/3x-ui/commit/8abe87b625686547bc68f9e3c3798cd2cb743515) | fix(outbounds): preserve stable subscription tags (#6345) | Server runtime |
| [`c62ee0bb`](https://github.com/MHSanaei/3x-ui/commit/c62ee0bbd8423fbb4523cc677dfddbedfc4a74cf) | fix(outbound): test VLESS vnext endpoints (#6358) | Server runtime |
| [`ac193cd9`](https://github.com/MHSanaei/3x-ui/commit/ac193cd9d389458a1f6aa7e7f7a363309d070da6) | refactor(ci): split the issue analyst out and brief the review job from a file | Docs / build / installation / tests |
| [`e264ea89`](https://github.com/MHSanaei/3x-ui/commit/e264ea89c1c7658147922a65e3b4759517414a42) | chore(deps): bump docs and frontend deps | Panel UI / frontend dependencies |
| [`38dd9bcc`](https://github.com/MHSanaei/3x-ui/commit/38dd9bcc703840739c509b1c54257e1e1994ae9a) | Bump Go dependency versions | Docs / build / installation / tests |
| [`65b9bfed`](https://github.com/MHSanaei/3x-ui/commit/65b9bfed8b8d5a8bf297da5cd9a1040c6bc34995) | fix(ci): stop the review bot handing over fixes in prose | Docs / build / installation / tests |
| [`e95fe80f`](https://github.com/MHSanaei/3x-ui/commit/e95fe80fc4f1717219f99b02143bc7e9960f8442) | fix(amneziawg): avoid manager lock inversion (#6397) | Server runtime |
| [`04e84580`](https://github.com/MHSanaei/3x-ui/commit/04e8458054b590e250a751adea70c37be8e4968e) | fix(frontend): improve dense QR readability (#6396) | Panel UI / frontend dependencies |
| [`0c72dd83`](https://github.com/MHSanaei/3x-ui/commit/0c72dd83841be53d05aeb3931273d19f1ba183bb) | fix(sub): restore compatible SOCKS subscription inbound (#6395) | API / server / subscription contract |
| [`ded2aa15`](https://github.com/MHSanaei/3x-ui/commit/ded2aa150ce6f4f8f3b7d320336635717d7ab1b2) | fix(frontend): isolate subscription language preference (#6394) | Panel UI / frontend dependencies |
| [`f9898e0b`](https://github.com/MHSanaei/3x-ui/commit/f9898e0b249dd727319e6cc32c8e49a95f035d76) | fix(sub): randomize fresh panel subscription paths (#6375) | Server runtime |
| [`540caa4e`](https://github.com/MHSanaei/3x-ui/commit/540caa4e935801c11fb6afda38e879eee9d312bc) | fix(hysteria): standard geco share links and persistent uTLS None (#6325) | API / server / subscription contract |
| [`23511108`](https://github.com/MHSanaei/3x-ui/commit/23511108bfe6919439cb2dea6d5b26fb8ac2737c) | fix(database): keep the SQLite store owner-only (#6390) | Server runtime |
| [`195988bd`](https://github.com/MHSanaei/3x-ui/commit/195988bdc138399e91c7acfa6d0d397c3b1e7e94) | fix(install): fetch x-ui.sh and unit files from the installed release tag (#6391) | Docs / build / installation / tests |
| [`de18c5a0`](https://github.com/MHSanaei/3x-ui/commit/de18c5a006e7fbbcb5fab3ab78cfc94e6017f411) | fix: do not type successfull login twice (#6374) | API / server / subscription contract |
| [`47964afb`](https://github.com/MHSanaei/3x-ui/commit/47964afbc5362cbaa182db98914cf63c343d6973) | fix(clients): render all tunnel configs for multi-inbound client (#6346) (#6349) | Panel UI / frontend dependencies |
| [`25d0c06f`](https://github.com/MHSanaei/3x-ui/commit/25d0c06f8923b827237984cfc15adefcf64b76c4) | fix(ci): skip a head the review bot already reviewed, and report a refused run | Docs / build / installation / tests |
| [`f9de0226`](https://github.com/MHSanaei/3x-ui/commit/f9de0226fe9ee533aee0c20e54470b6de0d66ab7) | fix(xray): confine log paths written under any key case | Server runtime |
| [`f17e4684`](https://github.com/MHSanaei/3x-ui/commit/f17e4684e0029307c2447c6dd32a153aa53341cd) | fix(sub): apply the device limit to ?view=raw | API / server / subscription contract |
| [`a31fa9ab`](https://github.com/MHSanaei/3x-ui/commit/a31fa9abfa4d316ff77608da0326c4ade08acd84) | fix(node): refuse a node's claim on another inbound's client | Server runtime |
| [`8411b1dd`](https://github.com/MHSanaei/3x-ui/commit/8411b1dd9ef6d32401d7146841990888010c59a9) | chore: upgrade Vitest to v5 | Panel UI / frontend dependencies |
| [`4019f47d`](https://github.com/MHSanaei/3x-ui/commit/4019f47de2c38cb8f02cb77c1766dbb1cc69adb1) | fix(x-ui.sh): put the fail2ban backend override in jail.d, not jail.conf (#6392) | Docs / build / installation / tests |
| [`f294e180`](https://github.com/MHSanaei/3x-ui/commit/f294e1806d083d8d394558452e2402878a9c94d2) | feat(release): publish SHA-256 sums and verify them in install.sh/update.sh (#6393) | Docs / build / installation / tests |
| [`0ff3c239`](https://github.com/MHSanaei/3x-ui/commit/0ff3c2394868ccb5ea1640e8ac3ae474f7732be3) | fix(api-docs): generate request bodies for all encodings (#6296) | Panel UI / frontend dependencies |
| [`bd1c27b0`](https://github.com/MHSanaei/3x-ui/commit/bd1c27b03de3f94ed623e5e7dd5483a03e48d41e) | fix(amneziawg): H1-H4 generator + queue-depth throughput fixes (#6330) | Server runtime |
| [`13e87a18`](https://github.com/MHSanaei/3x-ui/commit/13e87a18c86d2db227e2555aa677bae95d2813db) | chore(ci): give the race job a 25m test timeout | Docs / build / installation / tests |
| [`2ddcf530`](https://github.com/MHSanaei/3x-ui/commit/2ddcf5302077efb59d7b8a074dce23d53a429ecc) | Feature/fix external subscription client expiry (#6333) | API / server / subscription contract |
| [`63b46cd6`](https://github.com/MHSanaei/3x-ui/commit/63b46cd612c9e29a3968a716f36b6dbade4b8896) | perf(clients): apply a multi-inbound client create concurrently | API / server / subscription contract |
| [`41db85a0`](https://github.com/MHSanaei/3x-ui/commit/41db85a096d0dc6df008a69afc0bc0ad1f8e6fa2) | docs(claude): teach the bot briefings about AmneziaWG and PIA | Docs / build / installation / tests |
| [`f6bfcfe7`](https://github.com/MHSanaei/3x-ui/commit/f6bfcfe759f0ca02b9c45d7619347dcd7cd85e2c) | refactor(ci): make the Claude workflow review pull requests and nothing else | Docs / build / installation / tests |
| [`6f40a51d`](https://github.com/MHSanaei/3x-ui/commit/6f40a51d62b33f4fdefcadab0f3f82484d56cc85) | fix(node): sweep a selected inbound the node reports without its prefix | Server runtime |
| [`0775fcaa`](https://github.com/MHSanaei/3x-ui/commit/0775fcaad2ed9129916f1147487b2f4c3f71c782) | fix(node): keep an adopted inbound alias across a remote id cache refresh | Server runtime |
| [`ab422953`](https://github.com/MHSanaei/3x-ui/commit/ab4229534e8101ba458940732831897b741749fa) | fix(node): cap the status body the heartbeat probe decodes | Server runtime |
| [`5fc4b9f4`](https://github.com/MHSanaei/3x-ui/commit/5fc4b9f4633874d48cdc20f8e3ed8c07cbea0370) | fix(node): let a node-reported tag outrank a stale adopted alias | Server runtime |
| [`2e81865a`](https://github.com/MHSanaei/3x-ui/commit/2e81865a028a72fd276e337611e63a8b04d4ab45) | style(node): tighten the comments and probe assertion from the QA pass | Server runtime |
| [`3ef06b70`](https://github.com/MHSanaei/3x-ui/commit/3ef06b7000050cbd10504733000bfb2c8950fc04) | docs(readme): refresh all seven READMEs for the current feature set | Docs / build / installation / tests |
| [`d34ec97f`](https://github.com/MHSanaei/3x-ui/commit/d34ec97f62d3ee0a3ce8d6043f5a2921ea0e4010) | perf(node): push a client edit to every node at once, not one after another | Server runtime |
| [`24cb6bfe`](https://github.com/MHSanaei/3x-ui/commit/24cb6bfe1f2a9af52fb7b594854a31b48e18ce81) | perf(amneziawg): return gVisor's pooled buffers on the embedded data path | Server runtime |
| [`be5ee3e0`](https://github.com/MHSanaei/3x-ui/commit/be5ee3e0e17f3dd9931b7f84f6465b5cf17b8fd1) | fix(amneziawg): three defects in the embedded relay's connection handling | Server runtime |
| [`3b5273b1`](https://github.com/MHSanaei/3x-ui/commit/3b5273b1d6b2c55f15b5c75979077e116a62eadb) | fix(amneziawg): reject obfuscation values amneziawg-go's own UAPI rejects | Server runtime |
| [`ed6bc1d8`](https://github.com/MHSanaei/3x-ui/commit/ed6bc1d898a982d318ef653501ca0ab48d047460) | docs(api): align OpenAPI with runtime contracts (#6409) | API / server / subscription contract |
| [`0f6e1ae8`](https://github.com/MHSanaei/3x-ui/commit/0f6e1ae8d73bf6ae2d29aa346a76607fcbe1c53e) | fix(sub): bind JSON local inbounds to 127.0.0.1 and keep mux.cool off Vision outbounds (#6418) | API / server / subscription contract |
| [`4e355edc`](https://github.com/MHSanaei/3x-ui/commit/4e355edc1511a2eb7cb8a96a2644a8a8f68a0775) | fix(sub): skip AmneziaWG JSON entries (#6420) | API / server / subscription contract |
| [`fc05249e`](https://github.com/MHSanaei/3x-ui/commit/fc05249e0cb6c7f25ab78a5a3f7fcf684fb6da6c) | fix(geofile): verify downloaded geo databases against published digests (#6404) | Server runtime |
| [`8e13f8b1`](https://github.com/MHSanaei/3x-ui/commit/8e13f8b172ee8651e5f5cc45e886010ebf31047b) | fix(tgbot): suppress 'message not modified' warnings in Telegram edit calls (#6340) | Server runtime |
| [`33058c8e`](https://github.com/MHSanaei/3x-ui/commit/33058c8eed533f5315586bead06ff8137bf1273a) | fix(tgbot): use a token telego accepts in the edit-message tests | Docs / build / installation / tests |
| [`f072d044`](https://github.com/MHSanaei/3x-ui/commit/f072d0448d412203cc728274a4814dd374868053) | fix(clients): flag the restart a partly-applied edit or delete still needs | API / server / subscription contract |
| [`f2cf5899`](https://github.com/MHSanaei/3x-ui/commit/f2cf58994761fa4fe2521491b831f532ce9bb600) | fix(node): flag every hosting node before a client edit applies | Server runtime |
| [`e9e2e302`](https://github.com/MHSanaei/3x-ui/commit/e9e2e30278ecd1a9723a4137551f1393bed19286) | perf(clients): push a bulk client change to every node at once | Server runtime |
| [`a5e68f41`](https://github.com/MHSanaei/3x-ui/commit/a5e68f410fa949b98dd8bd19e18c2591f0757336) | perf(node): bound the per-client node push and fan out the traffic reset | Server runtime |
| [`2ec6c736`](https://github.com/MHSanaei/3x-ui/commit/2ec6c736130a9d38ef81a16b77f07a749aefd2f0) | feat(xray): update xray-core to v26.9.8 and adapt panel | Server runtime |
| [`2d151d76`](https://github.com/MHSanaei/3x-ui/commit/2d151d7648af8d996e069b70f7c6d27583f34a56) | Update deps and simplify parsing | Server runtime |
| [`d2ac3b4d`](https://github.com/MHSanaei/3x-ui/commit/d2ac3b4d7a107dd42a479af57bd597bed9d1a526) | fix(cli): let -getApiToken name the token it regenerates (#6405) | Server runtime |
| [`5a63d5d4`](https://github.com/MHSanaei/3x-ui/commit/5a63d5d468ac50e16e5b1dc8db87dc0cf6934e29) | fix(mtproto): use hosts for public share links (#6369) | API / server / subscription contract |
| [`3cd3836d`](https://github.com/MHSanaei/3x-ui/commit/3cd3836d771c978c805ad66edb4e412909938cc6) | fix(amneziawg): account for S4 junk in the default tunnel MTU (#6376) | API / server / subscription contract |
| [`b8597314`](https://github.com/MHSanaei/3x-ui/commit/b8597314f8f7cdb62b71ac2cc1b85621dbe366e2) | docs(api): mark collection responses nullable (#6430) | Panel UI / frontend dependencies |
| [`9f76a66d`](https://github.com/MHSanaei/3x-ui/commit/9f76a66dcfb44e40777a89b9ad332999bd327964) | feat(sub): add dummy info node and status configs for subscriptions (#6412) | API / server / subscription contract |
| [`47d23033`](https://github.com/MHSanaei/3x-ui/commit/47d2303334d5962621b0d2d849d44569aebfbf39) | fix(inbounds): reject missing TLS certificates before saving (#6429) | API / server / subscription contract |
| [`acf3603d`](https://github.com/MHSanaei/3x-ui/commit/acf3603dc886925c8674220ac3d9ae59aa6e01c9) | refactor(ci): review pull requests with one senior-engineer role | Docs / build / installation / tests |
| [`20d7f91c`](https://github.com/MHSanaei/3x-ui/commit/20d7f91c651fbadb1403525ac5d6c0482984d885) | refactor(ci): add an adversarial pass and name the analyst briefing | Docs / build / installation / tests |
| [`246d9207`](https://github.com/MHSanaei/3x-ui/commit/246d9207a5ca62127a40eb571d3eceaa38f1c4f8) | fix(sub): emit a bare host in Clash proxies | API / server / subscription contract |
| [`bc57548a`](https://github.com/MHSanaei/3x-ui/commit/bc57548a35e3cf8be1cf7083c94e26df27cb6a31) | fix(clients): withdraw the delete tombstone when the email is re-created | Server runtime |
| [`2004340d`](https://github.com/MHSanaei/3x-ui/commit/2004340d1d961d8e415145c506d0927b976c5136) | fix(inbounds): let a node-adopted inbound keep its own protocol on edit | Server runtime |
| [`65c5580e`](https://github.com/MHSanaei/3x-ui/commit/65c5580e7dabfd3ae5929a125e40b362c44c3500) | fix(clients): keep per-peer keys when a client spans several tunnel inbounds | Server runtime |
| [`4e423fa4`](https://github.com/MHSanaei/3x-ui/commit/4e423fa452cd82533d5fbc45fae735a7c16a0811) | fix(sub): drop Reality parameters when a host forces plain TLS | API / server / subscription contract |
| [`705b291d`](https://github.com/MHSanaei/3x-ui/commit/705b291d34db249f7444e1b12911a3f5c0c18344) | fix(amneziawg): stop losing an inbound and its server keys on the API path | Server runtime |
| [`c392f367`](https://github.com/MHSanaei/3x-ui/commit/c392f367e12b9956c0793e5868bd67db0b14a22c) | fix(dns): stop offering a port field that DoH entries discard | Panel UI / frontend dependencies |
| [`efc603f5`](https://github.com/MHSanaei/3x-ui/commit/efc603f59c4730dbbe58caab0819ad675f06aecc) | fix(settings): show the SMTP failure reason instead of a raw i18n key | Server runtime |
| [`cfd596a4`](https://github.com/MHSanaei/3x-ui/commit/cfd596a48935463501d2a87f77eeba7d8940827b) | fix(amneziawg): let a cleared header protection key reach a running device | Server runtime |
| [`d0edbcec`](https://github.com/MHSanaei/3x-ui/commit/d0edbcec81562d4b497428f97adb5325b2b45299) | feat(xray): update xray-core to v26.9.9 and follow the udpHop move | API / server / subscription contract |
| [`6ee74f20`](https://github.com/MHSanaei/3x-ui/commit/6ee74f20322250f10698405e15a85ad2e44fc5cb) | fix(amneziawgnet): wait for the client netstack goroutines before closing its device | Docs / build / installation / tests |
| [`33e6c2ec`](https://github.com/MHSanaei/3x-ui/commit/33e6c2ec0cb312dd7fa986f7b386bb46b8cad40b) | chore(deps): raise the swagger-ui-react js-yaml override to 4.3.2 | Panel UI / frontend dependencies |
| [`87420e3b`](https://github.com/MHSanaei/3x-ui/commit/87420e3bb1ecc546bc5e98e2c3a63ee35f620448) | fix(tgbot): close stale-inbound TOCTOU and contain handler panics (#6442) | Server runtime |
| [`8076d5ed`](https://github.com/MHSanaei/3x-ui/commit/8076d5edfaddbff02fa69288e3b913d717552a3d) | chore(frontend): bump React and Zod deps | Panel UI / frontend dependencies |
| [`876497db`](https://github.com/MHSanaei/3x-ui/commit/876497db6e0facc94bd3c016984b0e55072f6871) | feat(sub): add AmneziaWG proxy generation for Clash subscriptions (#6326) | API / server / subscription contract |
| [`d5ab84e8`](https://github.com/MHSanaei/3x-ui/commit/d5ab84e8d503785d7f00c43a68c6ab08ef41b10c) | feat(amneziawg): add AmneziaWG as an outbound protocol (#6320) | Server runtime |
| [`14566580`](https://github.com/MHSanaei/3x-ui/commit/145665802864b66abb00b1bde791ce21c3eb8da9) | feat(sub): add Happ client integration, routing presets, and app management (#6434) | API / server / subscription contract |
| [`ed5465d0`](https://github.com/MHSanaei/3x-ui/commit/ed5465d0f2f8f83820151fa3c0eb4d4ccf851840) | feat(clients): support setting HWID limit and MTProto ad-tag in bulk adjust (#6399) | API / server / subscription contract |
| [`2dd903ea`](https://github.com/MHSanaei/3x-ui/commit/2dd903ea8e7bd912406375b84b39d476f895a55d) | feat(sub): bake Happ/INCY routing profiles into the JSON subscription (#6402) | API / server / subscription contract |
| [`fc08b533`](https://github.com/MHSanaei/3x-ui/commit/fc08b533958496cbdbf23bdcd54427bb79c619db) | feat(ui): add global command palette (Ctrl+K) for fast navigation and search (#6352) | Server runtime |
| [`8b9cf260`](https://github.com/MHSanaei/3x-ui/commit/8b9cf260b6853ef2c417a2bc76d3e9d22556d5f2) | perf(clients): batch the client record lookup in bulk operations | Server runtime |
| [`02f2a63c`](https://github.com/MHSanaei/3x-ui/commit/02f2a63c53911fc5639e5217574820b3302ac58b) | refactor(tgbot): extract the shared numeric keypad builder | Server runtime |
| [`0fbdf0f9`](https://github.com/MHSanaei/3x-ui/commit/0fbdf0f9bf6fe593c6c672863167f0ec89f27a6d) | fix(ui): keep the empty-group placeholder legible in dark mode | Panel UI / frontend dependencies |
| [`3f1e52f0`](https://github.com/MHSanaei/3x-ui/commit/3f1e52f09ee0ac611fbf3f3f9b7a4294c421a158) | refactor(panel): drop two duplicated helpers | Server runtime |
| [`64b6e43e`](https://github.com/MHSanaei/3x-ui/commit/64b6e43e2b5d504a8034d33e9de224f3fbd1751e) | feat(sub): add legacy Clash subscription endpoint (#6338) | API / server / subscription contract |
| [`8f162994`](https://github.com/MHSanaei/3x-ui/commit/8f162994efcf033b95efb46f188fe2403d3449f5) | feat(clients): let admins set PersistentKeepalive on tunnel clients (#6377) | API / server / subscription contract |
| [`89ee1242`](https://github.com/MHSanaei/3x-ui/commit/89ee1242bd875192d3b80efbb3202bb727233e3e) | feat(sub): add read-only HWID device-slot status endpoint (#6380) | API / server / subscription contract |
| [`9f07951b`](https://github.com/MHSanaei/3x-ui/commit/9f07951ba78e42d81bb64929f6c335fd41aa3262) | feat(outbounds): support custom subscription user agents (#6398) | API / server / subscription contract |
| [`0a2cd789`](https://github.com/MHSanaei/3x-ui/commit/0a2cd789ba11a617a910f7c4d1bdbf30883aef39) | fix(sub): enable ML-KEM for Mihomo REALITY subscriptions (#6451) | API / server / subscription contract |
| [`6d96accd`](https://github.com/MHSanaei/3x-ui/commit/6d96accd6338e9ea3c7e5665415d489f2c116355) | Feature/tuic v5 (#6337) | API / server / subscription contract |
| [`5815254f`](https://github.com/MHSanaei/3x-ui/commit/5815254fc3d93338b4042eab9f58d6c3462e2201) | fix(tuic): evict the oldest relay flow instead of refusing new clients | Server runtime |
| [`19a692e0`](https://github.com/MHSanaei/3x-ui/commit/19a692e074d1163453b76de9af4d90777375b21c) | fix(install): stop copying tuic-server over /usr/local/bin | Docs / build / installation / tests |
| [`dd46a067`](https://github.com/MHSanaei/3x-ui/commit/dd46a0676167cf7a921d40bf01edc430210312d9) | Update deps and fix AntD Space API | Panel UI / frontend dependencies |
| [`f51b0040`](https://github.com/MHSanaei/3x-ui/commit/f51b0040cf88c30dbc1515ceb0e5b0d79948bf61) | fix(link): map vcn to verifyPeerCertByName in applySecurity (#6479) | Server runtime |
| [`5cce2464`](https://github.com/MHSanaei/3x-ui/commit/5cce2464f11fd3e922b48cf7d4487ea85e537fb2) | fix(clients): snap EOM 23:59:59 expiry to billing midnight without renew (#6457) | Server runtime |
| [`22763fe8`](https://github.com/MHSanaei/3x-ui/commit/22763fe8f64e8f48e1c07df95fcad391e30a7a3d) | feat(clients): add Generate button for WireGuard/AmneziaWG PresharedKey (#6455) | Panel UI / frontend dependencies |
| [`7a41c594`](https://github.com/MHSanaei/3x-ui/commit/7a41c5949406089b441c7ff73d464144931f51a9) | fix(amneziawg): honor inbound listen when binding UDP socket (#6461) | Server runtime |
| [`67addab3`](https://github.com/MHSanaei/3x-ui/commit/67addab343ac6955eb552d63d23e379486c8b392) | fix(inbounds): serve fresh client UUIDs for list and allLinks (#6458) | API / server / subscription contract |
| [`3a932357`](https://github.com/MHSanaei/3x-ui/commit/3a93235783b8c08945a778ef595263b567c78d71) | docs(readme): add 3X-UI Manager to Community Tools (#6266) | Docs / build / installation / tests |
| [`0a838563`](https://github.com/MHSanaei/3x-ui/commit/0a838563bb9f2eeb96432843514d9a01b176d565) | fix(clients): use EffectiveFlow in BulkAttach (#6454) | Server runtime |
| [`8082ab4d`](https://github.com/MHSanaei/3x-ui/commit/8082ab4d749fc70156f700e9b5cd122ed41228e0) | feat(clients): show short HWID fingerprint in admin device list (#6464) | Server runtime |
| [`5fbd2b49`](https://github.com/MHSanaei/3x-ui/commit/5fbd2b490c2078e7a56db8bb063fe5093d366d37) | fix(clients): preserve enable on portable import (#6481) | Server runtime |
| [`b467d4c6`](https://github.com/MHSanaei/3x-ui/commit/b467d4c676ad16efe98c75967207c0a0a35c50cc) | feat(reality): warn when target cert chain is too small for ML-DSA-65 (#6470) | Server runtime |
| [`51e0afdd`](https://github.com/MHSanaei/3x-ui/commit/51e0afdd908df7325c118f06c057f629317be48d) | fix(inbounds): allow negative subSortIndex for subscription order (#6465) | API / server / subscription contract |
| [`b332d884`](https://github.com/MHSanaei/3x-ui/commit/b332d88438032efa68a49cb59278aeff6968cf84) | feat(settings): add Block tab for JSON subscription routing rules (#6466) | Panel UI / frontend dependencies |
| [`6a159683`](https://github.com/MHSanaei/3x-ui/commit/6a159683d5892f9262da724cc16054eabd317e94) | docs: update star history badges | Docs / build / installation / tests |
| [`958d7f13`](https://github.com/MHSanaei/3x-ui/commit/958d7f138ec68cba38f9e724533fa3dcb28daec9) | fix(frontend): fold sockopt v6only into V6Only on inbound load (#6453) | Panel UI / frontend dependencies |
| [`bdd351bd`](https://github.com/MHSanaei/3x-ui/commit/bdd351bd157ec0321cfed8ae62b4c120ea28c44d) | fix(api): return 401 for invalid Bearer token instead of 404 (#6459) | API / server / subscription contract |
| [`503b5df4`](https://github.com/MHSanaei/3x-ui/commit/503b5df4b961c515a88390b2ae51045f53ab223e) | fix(link): preserve Shadowsocks TLS query params on import (#6467) | Server runtime |
| [`72df05a4`](https://github.com/MHSanaei/3x-ui/commit/72df05a403e2c2c0126e939a1fb8dc3a9fb0c5b9) | fix(hosts): keep TLS override fields visible when Security is same (#6452) | Panel UI / frontend dependencies |
| [`7ac5277c`](https://github.com/MHSanaei/3x-ui/commit/7ac5277c4f2657e08edda508e293e09588201e0f) | fix(tgbot): require client ownership for non-admin link callbacks (#6489) | Server runtime |
| [`615876b2`](https://github.com/MHSanaei/3x-ui/commit/615876b2eb0b9ad16b0722aca803f7dbe3e76578) | fix(tgbot): read the admin list and running flag under their mutex (#6491) | Server runtime |
| [`02c6c3a9`](https://github.com/MHSanaei/3x-ui/commit/02c6c3a9c653d41674511e6e21fcc21ac61d2d93) | fix(tgbot): render the add-client draft as HTML and escape its values (#6492) | Server runtime |
| [`aaa5e61c`](https://github.com/MHSanaei/3x-ui/commit/aaa5e61cad68772885f4b9b6afa1e98a8f5f4c8d) | fix(tgbot): answer the callbacks the bot cannot route (#6493) | Server runtime |
| [`2730e4d0`](https://github.com/MHSanaei/3x-ui/commit/2730e4d071ae7d05225d44930463fccfa89b630e) | feat(sub): let the panel set the JSON subscription DNS servers (#6485) | API / server / subscription contract |
| [`b98f947e`](https://github.com/MHSanaei/3x-ui/commit/b98f947efe12054165594fd821c1c6c55bfa196b) | fix(tgbot): answer only the link callbacks that match nothing | Server runtime |
| [`c3b08b6d`](https://github.com/MHSanaei/3x-ui/commit/c3b08b6d9f1761736b14dbd3c44056d94f441810) | fix(tgbot): guard the mock Telegram server's call counts | Docs / build / installation / tests |
| [`6a5b4fab`](https://github.com/MHSanaei/3x-ui/commit/6a5b4fab6acbc38f435abaa86f639e63b5a537b9) | feat(happ): generate Crypt5 subscription links locally (#6494) | API / server / subscription contract |
| [`c7518c40`](https://github.com/MHSanaei/3x-ui/commit/c7518c4038bb81eeebacf1e9a0d4dfd746626a96) | fix(tgbot): send the admin traffic reports as one message (#6490) | Server runtime |
| [`cba8f067`](https://github.com/MHSanaei/3x-ui/commit/cba8f0672f6e665f48ed45ce1c2c04f6e30caf5a) | feat(sub): refine Happ routing presets, serverDescription escaping, and auto-detect placement (#6488) | API / server / subscription contract |
| [`bf7ce2da`](https://github.com/MHSanaei/3x-ui/commit/bf7ce2daaac3de29f2706014c78b00c4b18828c9) | feat(discord): add Discord notification bot service (#6486) | API / server / subscription contract |
| [`768bbd2a`](https://github.com/MHSanaei/3x-ui/commit/768bbd2a292edd79bb231e409e1e58efb9eccbd6) | feat(settings): add setting for Reality scan candidates (#6471) | API / server / subscription contract |
| [`cc60cefe`](https://github.com/MHSanaei/3x-ui/commit/cc60cefe0248f4142708fd9126b99c2180a6ce24) | fix(discord): report a start-after-first-use client as days, not unlimited (#6498) | Server runtime |
| [`5c34baa8`](https://github.com/MHSanaei/3x-ui/commit/5c34baa8df33120676f92143cdc3c53056eb5f05) | fix(discord): drop the gateway connection when heartbeats go unanswered (#6497) | Server runtime |
| [`d45a09d6`](https://github.com/MHSanaei/3x-ui/commit/d45a09d634a58d2c0c8ab2eaf17e4ed22d27ddf1) | fix(discord): page the inbounds reply within Discord's embed caps (#6496) | Server runtime |
| [`e98be4f7`](https://github.com/MHSanaei/3x-ui/commit/e98be4f72adbb735bc4682b2b66877ea294f3875) | fix(tgbot): render a disabled start-after-first-use client as days (#6500) | Server runtime |
| [`1691c9ca`](https://github.com/MHSanaei/3x-ui/commit/1691c9ca2ada553abb3ca77f8378a43421f71a59) | fix(tgbot): keep the add-client draft with the chat that owns it (#6499) | Server runtime |
| [`2fcd28c1`](https://github.com/MHSanaei/3x-ui/commit/2fcd28c1bce59f6cbfcdd079addff23bcef29ee7) | refactor(tgbot): make the add-client expiry presets say what they do (#6503) | Server runtime |
| [`f3dba07e`](https://github.com/MHSanaei/3x-ui/commit/f3dba07e13a7d59cb4837ff37e17dc8b21ea97e4) | fix(link): read the vmess certificate checks on import (#6507) | Server runtime |
| [`8fc4fc0b`](https://github.com/MHSanaei/3x-ui/commit/8fc4fc0bf8d87b1d4eb9dee55737631d7eb00c48) | fix(link): rebuild shadowsocks tcp/http obfuscation on import (#6505) | Server runtime |
| [`ff1a6c3c`](https://github.com/MHSanaei/3x-ui/commit/ff1a6c3caffaea76bdabeb36d42a070e90e35f5c) | fix(sub): drop external Clash shadowsocks nodes the panel cannot express (#6508) | API / server / subscription contract |
| [`d600de2c`](https://github.com/MHSanaei/3x-ui/commit/d600de2c2e8aee709687ae82ada54a1decb6f11c) | feat(geodata): add standard source presets (#6504) | API / server / subscription contract |
| [`d0ad773e`](https://github.com/MHSanaei/3x-ui/commit/d0ad773edf116ace8b9336ee5f8c37f75d6b6af2) | fix(clients): preserve traffic reset schedule when toggling enable (#6502) | Panel UI / frontend dependencies |
| [`435ed976`](https://github.com/MHSanaei/3x-ui/commit/435ed976c07825e2a565b0eb4b8eeb126f316e69) | fix(web): restart panel after ImportDB so subPath routes match (#6446) (#6456) | API / server / subscription contract |
| [`4760ccab`](https://github.com/MHSanaei/3x-ui/commit/4760ccaba0396e0e6e617c00c2aa20b8e54c111d) | fix(logs): standardize logs (#6484) | API / server / subscription contract |
| [`939c4706`](https://github.com/MHSanaei/3x-ui/commit/939c4706984ac4ac30dcb3b05225f5b35ef12158) | feat(inbounds): show linked host remarks in inbound list (#6468) | Panel UI / frontend dependencies |
| [`5ad9df69`](https://github.com/MHSanaei/3x-ui/commit/5ad9df69b9af3a216ac6a040b3172b39e76a4fc8) | fix(link): restore mKCP seed and headerType on share-link import (#6480) | Server runtime |
| [`22346eef`](https://github.com/MHSanaei/3x-ui/commit/22346eef78071c5e25ecccfa54f2c68f0c290b7b) | fix(node): import a newly selected node inbound instead of sweeping it | API / server / subscription contract |
| [`cfd4f64a`](https://github.com/MHSanaei/3x-ui/commit/cfd4f64a79214a4ad1b06342c9e97b3a81297459) | fix(amneziawg): bound the SOCKS5 UDP associate exchange | Server runtime |
| [`2d7c8c77`](https://github.com/MHSanaei/3x-ui/commit/2d7c8c77f71d3698c444ab37acf1f72257e945b8) | docs: add TUIC v5 to READMEs, guides, and protocol references (#6511) | Docs / build / installation / tests |
| [`a09e1360`](https://github.com/MHSanaei/3x-ui/commit/a09e136001b877510048b95d5d3bfb381c9d284b) | docs: add Discord bot to READMEs, architecture, operations guides, and locales (#6513) | Docs / build / installation / tests |
| [`032ddcb2`](https://github.com/MHSanaei/3x-ui/commit/032ddcb29fb6b42d044ebefb76bf773ee5b1dc63) | fix(nodetoken): make the corrupt-ciphertext test corrupt deterministically (#6520) | Docs / build / installation / tests |
| [`826e29e2`](https://github.com/MHSanaei/3x-ui/commit/826e29e2de245b73b4da6ecf6b398cb623effda8) | fix(xray): place the freedom domain strategy where the core reads it (#6515) | Server runtime |
| [`c90996ed`](https://github.com/MHSanaei/3x-ui/commit/c90996eda34146668dfc379c2b5637f097e51d9d) | feat(sub): add opt-in month-end expiry presentation (#6517) | API / server / subscription contract |
| [`39ce7cbc`](https://github.com/MHSanaei/3x-ui/commit/39ce7cbc22b737f5ac78ce89071164230083978e) | fix(xray): migrate the dns outbound off its legacy nonIPQuery and blockTypes (#6519) | Server runtime |
| [`c0c2dd27`](https://github.com/MHSanaei/3x-ui/commit/c0c2dd274c22a8ffd47cd8048354109ab1f05b5f) | fix(panel): read outbound protocol ids case-insensitively everywhere (#6523) | Panel UI / frontend dependencies |
| [`a5a4c9cd`](https://github.com/MHSanaei/3x-ui/commit/a5a4c9cd831085f476126afdb792434c383a1647) | fix(panel): read an outbound protocol id the way the core does (#6522) | Panel UI / frontend dependencies |
| [`84c5aef4`](https://github.com/MHSanaei/3x-ui/commit/84c5aef4a111f95c92280e91f78ad38c239b3a47) | fix(panel): probe UDP outbounds and hide the block outbound from the mtproto egress picker (#6525) | Panel UI / frontend dependencies |
| [`4d6db1c9`](https://github.com/MHSanaei/3x-ui/commit/4d6db1c961a3e7bb880425ac6e01e8ac5835e049) | fix(xray): read an outbound protocol id the way the core does (#6521) | Server runtime |
| [`f69d1e86`](https://github.com/MHSanaei/3x-ui/commit/f69d1e869dbc7afdd7f0fa6d0f3b9726236e0118) | fix(outbound): read the probe protocol id and transport name like the core (#6526) | Server runtime |
| [`efcf1529`](https://github.com/MHSanaei/3x-ui/commit/efcf152950f51f15a1721c7cca1754651d21166a) | fix(outbound): read the probe testability gate's ids like the core (#6527) | Server runtime |
| [`c0271e23`](https://github.com/MHSanaei/3x-ui/commit/c0271e231dfb649539d31517256f9246fec7c23f) | fix(panel): read the outbound protocol id in the Outbounds row like the core (#6528) | Panel UI / frontend dependencies |
| [`840a40ed`](https://github.com/MHSanaei/3x-ui/commit/840a40edcd8e2ffbcdd3a337dfd099e22b6e79f0) | chore(deps): update frontend and Go deps | Panel UI / frontend dependencies |
| [`837addf6`](https://github.com/MHSanaei/3x-ui/commit/837addf66e945a80080273b5d2a315dea765d748) | v3.8.0 | Server runtime |

### v3.8.0 → v3.8.5

| Commit | Change | Area |
| --- | --- | --- |
| [`a810f497`](https://github.com/MHSanaei/3x-ui/commit/a810f497e6ed3915ba58413dd64a64e619f987ad) | fix(xray): read the last two inboundTag protocol ids like the core (#6530) | Server runtime |
| [`78ab7a92`](https://github.com/MHSanaei/3x-ui/commit/78ab7a924600c5c6de9692941f0478188f99a657) | fix(amneziawg): read the outbound pseudo-protocol id like the core (#6531) | Server runtime |
| [`a036ddd6`](https://github.com/MHSanaei/3x-ui/commit/a036ddd66f44f4bfe437fba83bbf0d7b6c3adbc5) | fix(amneziawg): wrap the relay port window instead of refusing ids past it (#6539) | Server runtime |
| [`2d8d3048`](https://github.com/MHSanaei/3x-ui/commit/2d8d304850f718b462a1b473dc5b7ceedc8cbef4) | fix(amneziawg): stop a disabled inbound's relay slot from being taken (#6540) | Server runtime |
| [`d52b598a`](https://github.com/MHSanaei/3x-ui/commit/d52b598abf3e38a3936c8b859eee812dfc3468f9) | fix(amneziawg): reserve the relay port before an AmneziaWG inbound has a peer (#6542) | Server runtime |
| [`43e64993`](https://github.com/MHSanaei/3x-ui/commit/43e64993fc9a760a74630cc78ec1e6a68b61a7dc) | fix(amneziawg): refuse a row's own relay port and keep a disabled row's slot reserved (#6544) | Server runtime |
| [`baef3cdd`](https://github.com/MHSanaei/3x-ui/commit/baef3cdd070b87e7abade37b0e469526023921e5) | fix(xray): refuse a config the running core cannot bind (#6547) | Server runtime |
| [`574caa63`](https://github.com/MHSanaei/3x-ui/commit/574caa63e957153e2c862d420fcb3c2acf27894b) | fix(inbounds): check ports when an inbound is enabled, not only when it is saved (#6549) | Server runtime |
| [`4a8fdcee`](https://github.com/MHSanaei/3x-ui/commit/4a8fdceed633ae6066f1a17cfe9080dfdf310986) | perf(nodes): reuse one pooled client per node instead of rebuilding it (#6548) | Server runtime |
| [`d089adee`](https://github.com/MHSanaei/3x-ui/commit/d089adeeea2ed436ccfc9d0ac90e2c27c5f4bb20) | docs(limit-ip): correct what the temporary disconnect can actually do (#6551) | Server runtime |
| [`e790f467`](https://github.com/MHSanaei/3x-ui/commit/e790f4675769478a6e340275e8747012621a81f7) | fix(xray): restart when a diff strands a client's live session (#6550) | Server runtime |
| [`d440c2b9`](https://github.com/MHSanaei/3x-ui/commit/d440c2b932e29a014d63888f2a46d051f9725dfa) | fix(panel): accept 2FA codes from adjacent TOTP windows (#6546) | Server runtime |
| [`d9c7c76f`](https://github.com/MHSanaei/3x-ui/commit/d9c7c76fb0592bfa46d7176f0c8ecc3751887bac) | fix(limit-ip): leave a reverse client out of the temporary disconnect (#6553) | Server runtime |
| [`ac3fc120`](https://github.com/MHSanaei/3x-ui/commit/ac3fc12077ef7c5fba45226a1885f13669b34cb9) | fix(ports): refuse an inbound on a port an AmneziaWG peer forwards (#6554) | Server runtime |
| [`bc424f09`](https://github.com/MHSanaei/3x-ui/commit/bc424f096837c7155cc677532423b12837186623) | fix(xray): stop a lone dns qType 0 from matching every query | Server runtime |
| [`789a0306`](https://github.com/MHSanaei/3x-ui/commit/789a03065aece992bfc90207b8735c2bc19bb8e3) | chore(docs): bump dependencies and adapt to fumadocs-core 16.15.11 | Docs / build / installation / tests |
| [`af466b6a`](https://github.com/MHSanaei/3x-ui/commit/af466b6a244d126ce85f28f8c08730279e9ccedc) | fix(node): push a node only the client IPs it hosts | Server runtime |
| [`cfa8350d`](https://github.com/MHSanaei/3x-ui/commit/cfa8350d10adf343f4611ac685246860f9aebb42) | fix(clients): keep a vless reverse client's handler across a re-add (#6558) | Server runtime |
| [`ea66aa49`](https://github.com/MHSanaei/3x-ui/commit/ea66aa4971576e8015717dc95c17bb9f8bbd4024) | fix(traffic): push depletion changes to nodes off the serial writer | Server runtime |
| [`a84bbeab`](https://github.com/MHSanaei/3x-ui/commit/a84bbeab2eb6e5b0329c6399bb66c2605839faf2) | fix(node): drop online clients and sub-nodes of nodes no longer synced | Server runtime |
| [`eb11e8c8`](https://github.com/MHSanaei/3x-ui/commit/eb11e8c85ad12a660bc8c855730872a38ef29ba5) | fix(node): fan out operations that call every node | Server runtime |
| [`56bb876d`](https://github.com/MHSanaei/3x-ui/commit/56bb876d8dce6d9a20d205d7835f5e0c5b14e634) | fix(node): send one alert for a burst of node transitions | Server runtime |
| [`dea7cd9c`](https://github.com/MHSanaei/3x-ui/commit/dea7cd9cc19856743eb2f045a3a77f77e027209d) | fix(traffic): reset due inbounds and clients concurrently | Server runtime |
| [`3c8cf357`](https://github.com/MHSanaei/3x-ui/commit/3c8cf35734f27cd7acda8cbc6725b53bed471e42) | perf(node): sync up to 32 nodes at once, like the heartbeat | Server runtime |
| [`bc49c1a6`](https://github.com/MHSanaei/3x-ui/commit/bc49c1a68f574e12a770a711efb9bc184bfa0a06) | fix(node): release a deleted node's metric series and HTTP client | Server runtime |
| [`d1c4e026`](https://github.com/MHSanaei/3x-ui/commit/d1c4e0261bff5f24f20cae365ec860ecbfb466e4) | chore(node): cover the sync tick's online prune from the job package | Server runtime |
| [`3c1498d8`](https://github.com/MHSanaei/3x-ui/commit/3c1498d806246b3dd1dcb314d08b998116e01c03) | fix(ldap): apply LDAP enable, disable and cleanup through the bulk paths | Server runtime |
| [`7fc86f87`](https://github.com/MHSanaei/3x-ui/commit/7fc86f87de7befe5228767c4f523524760d9219f) | perf(inbounds): keep unchanged rows and online sets across websocket pushes | Panel UI / frontend dependencies |
| [`3fa44915`](https://github.com/MHSanaei/3x-ui/commit/3fa44915c1b2f814ddbec494c405221b921ab2f5) | perf(nodes): keep the node table element across unrelated re-renders | Panel UI / frontend dependencies |
| [`1d85ef13`](https://github.com/MHSanaei/3x-ui/commit/1d85ef138ed823969c80409af596dd9400df6c10) | fix(sub): prevent default profile page URL disclosure (#6538) | API / server / subscription contract |
| [`14b92fbc`](https://github.com/MHSanaei/3x-ui/commit/14b92fbcff9ded9846c4ddadb2fea51752a4d202) | fix(nodes): stop flagging a node on the other update channel as outdated | Panel UI / frontend dependencies |
| [`e8bab17c`](https://github.com/MHSanaei/3x-ui/commit/e8bab17c2fed26d54781b0eb73374ed18122f13b) | fix(clients): stop the Edit Client modal showing a stray light scrollbar | Panel UI / frontend dependencies |
| [`5fe4f241`](https://github.com/MHSanaei/3x-ui/commit/5fe4f241c11ad02660d975a75739978691ffe437) | style(logs): widen the row-count selector in the log modals | Panel UI / frontend dependencies |
| [`5008906c`](https://github.com/MHSanaei/3x-ui/commit/5008906c4c7977ea3c5957b7df507219d116e613) | feat(clients): filter the client list by clicking a summary stat card | Server runtime |
| [`c9e62451`](https://github.com/MHSanaei/3x-ui/commit/c9e62451e6238e85fbd037fe61452a3ce39a32cf) | fix(outbounds): keep subscription tags on their server when reality params rotate | Server runtime |
| [`01ce2bce`](https://github.com/MHSanaei/3x-ui/commit/01ce2bcecbdfc19dc5e9776d05e6370fbf413794) | feat(api-docs): split the API docs page into tabs | Panel UI / frontend dependencies |
| [`e26cf1d3`](https://github.com/MHSanaei/3x-ui/commit/e26cf1d3ed11b9bf2b796c0ec36ce5aff9e856c2) | feat(sub): redesign the subscription page around usage, tabs and app imports | API / server / subscription contract |
| [`ec9fbae6`](https://github.com/MHSanaei/3x-ui/commit/ec9fbae645631a1e3341278183f5ea51da29bc00) | v3.8.5 | Server runtime |
| [`7ef22f94`](https://github.com/MHSanaei/3x-ui/commit/7ef22f94c950ff09f0870e2295fa65ad5968742c) | fix(logger): fix data race in InitLogger | Server runtime |
