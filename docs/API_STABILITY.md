# Public API Boundary And Stability Policy

`rnk` is still pre-1.0. The project should be useful today, but callers should
have one clear import path and one clear set of expectations for what can change
while the framework matures.

## Stable User Surface

The recommended stable user surface is:

```rust
use rnk::prelude::*;
```

The prelude is the primary API for applications. It contains the public types,
components, hooks, and render entry points that examples and user docs should
prefer.

The root crate also re-exports a small compatibility surface:

- `rnk::Box`
- `rnk::Text`
- `rnk::Color`
- `rnk::Element`
- `rnk::ElementId`
- `rnk::Style`
- render entry points such as `rnk::render`, `rnk::render_inline`, and
  `rnk::render_fullscreen`
- cross-thread and persistent-output helpers such as `rnk::request_render` and
  `rnk::println`
- declarative UI macros such as `row!`, `col!`, `box_element!`,
  `text!`, `styled_text!`, `spacer!`, `when!`, `list!`, and
  `list_indexed!`

Those root re-exports are supported convenience imports, but new examples should
prefer `rnk::prelude::*` unless a narrow import is clearer.

`rnk::prelude::lite::*` is the stable low-conflict import set for small examples
or applications that want fewer names in scope.

Some renderer-adjacent types are intentionally in the prelude because app code
uses them directly:

| Prelude item | Stability |
| --- | --- |
| `AppBuilder`, `AppOptions`, `render`, `render_inline`, `render_fullscreen` | Stable application configuration and entry points. |
| `RenderOptions` and `render_to_string*` helpers | Stable test/snapshot helpers, with additive options expected. |
| `RenderHandle`, `request_render`, `println`, `println_trimmed`, `enter_alt_screen`, `exit_alt_screen`, `is_alt_screen` | Stable app-control helpers for current runtime modes. |
| `ModeSwitch`, `Printable`, `IntoPrintable` | Advanced but prelude-exported compatibility types; variants may grow before `1.0`. |

Types that are public only through `rnk::renderer`, and not listed above or in
the root compatibility surface, remain advanced or experimental.

## Advanced And Experimental Surfaces

The following modules are public because they are useful for advanced users,
tests, or extension work. They are not the preferred application surface:

| Module | Status | Notes |
| --- | --- | --- |
| `rnk::core` | Advanced | Core element, style, color, and layout primitives. Prefer the prelude for normal apps. |
| `rnk::components` | Advanced | Component modules and concrete component types. Prefer prelude re-exports in examples. |
| `rnk::hooks` | Advanced | Hook implementation and hook types. Prefer prelude re-exports for app code. |
| `rnk::renderer` | Experimental advanced | Renderer controls, output buffers, terminal abstraction, frame-rate types, and render-to-string helpers. |
| `rnk::runtime` | Experimental internal-adjacent | Terminal/runtime state utilities. Useful for integration work, but not a stable app contract. |
| `rnk::testing` | Experimental test support | Test renderer, harness, assertions, golden helpers, and generators. These may grow as #27 improves the harness. |
| `rnk::cmd` | Advanced | Side-effect command model. Public because apps can build commands, but details may evolve. |
| `rnk::animation` | Advanced | Animation primitives used by components and applications. |
| `rnk::layout` | Advanced | Layout measurement utilities and Taffy-backed behavior details. |
| `rnk::macros` and exported UI macros | Stable convenience surface | `row!`, `col!`, `box_element!`, `text!`, `styled_text!`, `spacer!`, `when!`, `list!`, and `list_indexed!` are root-exported declarative helpers and may gain additive forms. |
| `impl_into_element!` | Advanced extension macro | Root-exported today for component authors. It may change before `1.0` if component conversion rules are tightened. |
| `golden_test!`, `inline_snapshot!`, `assert_snapshot!` | Experimental test macros | Root-exported test helpers. #27 may revise names, output formats, update behavior, or macro arguments. |
| `rnk::reconciler` | Hidden internal | Marked `#[doc(hidden)]`; callers should not depend on it. |

Public modules not listed as stable should be treated as pre-1.0 advanced
interfaces. They can change when the change is needed to stabilize the prelude,
fix terminal behavior, or complete the maturity spec.

## Public Field Audit

Some public structs expose fields today. Until `1.0`, public fields are governed
by these rules:

- Fields on prelude-facing user types should not be removed or have their meaning
  changed without a migration note.
- New fields may be added before `1.0` when they have a default, builder, or
  obvious non-breaking construction path.
- Internal bookkeeping fields should be `#[doc(hidden)]` or moved behind methods
  before `1.0`.
- Direct field construction remains supported for ergonomic types where the
  project intentionally mirrors CSS or terminal primitives.

Current field exposure review:

| Type | Current field posture | Stability decision |
| --- | --- | --- |
| `Element` | Public tree fields such as `id`, `element_type`, `style`, `children`, text, key, and scroll offsets. | Advanced but supported before `1.0`; prefer constructors/builders in examples. Field set may be reduced or moved behind methods before `1.0`. |
| `Style` | Public CSS-like layout, spacing, border, color, text, overflow, and `#[doc(hidden)] is_static`. | Intentionally field-addressable for style construction and tests. New token/state fields may be added by #24. |
| `Theme` | Public semantic color fields and component color groups. | Field layout is preserved for direct construction. Non-color tokens are currently resolved through `Theme::design_tokens()` and `Theme::variant_style(...)` instead of new public fields. |
| `AppOptions` | Public renderer options. | Supported configuration struct. New fields may be added with defaults. |
| `RenderOptions` | Public render-to-string options. | Supported for testing and snapshots; may gain options as terminal compatibility work expands. |
| `StyledChar`, `ClipRegion`, and `Output` | Public renderer buffer fields such as cells, clip coordinates, width, and height. | Advanced/experimental renderer internals. Direct field construction is allowed today for diagnostics and tests, but fields may be hidden or replaced by accessors before `1.0`. |
| `FrameRateConfig` and `FrameRateStats` | Public frame-rate configuration and statistics fields. | Advanced app-control API. Additive fields are expected; existing field meanings should be migrated with release notes if changed. |
| `FilterResult`, `EventFilter`, and `FilterChain` | Mostly private fields, public composition methods. | Advanced input-filter API. The enum may gain variants before `1.0`. |
| `Terminal` | Public terminal abstraction with private fields. | Experimental renderer type; prefer `render*` entry points for app code. |
| `RuntimeContext` | Fields are private; methods expose runtime behavior. | Experimental internal-adjacent type. Public methods may change before `1.0` as runtime ownership is stabilized. |
| `Environment` | Public runtime environment detection fields for CI, TTY, and terminal size. | Advanced runtime API. Additive fields are expected as terminal compatibility work expands. |
| `TestHarness`, `TestRenderer`, `GoldenTest`, `Snapshot`, `StringSnapshot` | Public test support, mostly private fields. | Experimental test API. #27 is expected to add interaction helpers and may adjust names. |
| `LayoutError` | Public testing enum with payload-bearing variants such as coordinates, dimensions, bounds, and Unicode width diagnostics. | Experimental test API. Variants or payload fields may change before `1.0`; downstream code should avoid exhaustive matches unless pinned to a minor version. |
| `GoldenResult` | Public testing enum; `Mismatch` exposes `expected`, `actual`, and `diff` strings. | Experimental test API. #27 may add variants or change mismatch payload shape as golden support is expanded. |
| `UnicodeWidthTestCase` and generator helpers | Public testing fixture fields and helper functions. | Experimental test-support API; useful for library tests but not a stable application contract. |
| `RenderHandle` | Private field, public app-control methods. | Stable app-control helper while current runtime mode APIs remain supported. |
| `rnk-style::Style` and `rnk-style-core` primitives | Helper-crate public APIs. | Independently versioned helper APIs. They follow their crate versions and may not release on the same cadence as `rnk`. |
| `rnk-icons::Icon` and `rnk-icons::icons::*` | Helper-crate public APIs. | Independently versioned icon API. Additive icon/function additions are expected. |

## Semver Before 1.0

While the crate version is `0.x`:

- Patch releases should be bug fixes, documentation, CI/release fixes, additive
  APIs, and compatible behavior clarifications.
- Minor releases may include breaking changes when they are required to stabilize
  the prelude, component contracts, renderer/runtime behavior, or test harness.
- Breaking changes should be documented in `CHANGELOG.md` or release notes with a
  migration note when the old behavior was public.
- The maturity issues (#22-#27) may change advanced/experimental APIs. They
  should preserve or clearly migrate the prelude whenever practical.
- The declared MSRV is part of the public contract. Raising it requires a release
  note and matching Cargo metadata/docs updates.

After `1.0`, normal semver rules apply: breaking public API changes require a
major version bump.

## How New APIs Should Be Added

When adding public APIs:

1. Prefer adding them to implementation modules first.
2. Re-export through `rnk::prelude` only after the API is intended for app users.
3. Document whether the API is stable, advanced, or experimental.
4. Add examples and tests for user-facing behavior.
5. Avoid exposing internal state as public fields unless direct construction is
   a deliberate ergonomic goal.

When in doubt, keep the API module-public but out of the prelude until the
contract is proven by examples and tests.
