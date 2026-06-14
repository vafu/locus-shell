# Agent Approvals Migration

## AGS Sources Reviewed

- `/home/v47/.config/ags/widgets/agent-approvals/index.ts`
- `/home/v47/.config/ags/widgets/agent-approvals/overlay.tsx`
- `/home/v47/.config/ags/services/agent.ts`
- `/home/v47/.config/ags/services/requests.ts`
- `/home/v47/.config/ags/app.ts`
- `/home/v47/.config/ags/style/agent-approvals.scss`
- `/home/v47/.config/ags/style/agent.scss`

## Target Ownership

Implement this as a user-facing `rsynapse-shell` overlay widget, not in
`shell/core`. `shell/core` should only provide the generic layer-shell window,
CSS registration, provider runtime, and Relm4 integration primitives.

The widget should own:

- Agent approval overlay role and layer-shell placement.
- Agent D-Bus session contract and response methods, unless that contract is
  promoted later into a consumer-local provider module.
- Derived pending approval list.
- Manual visibility state and target-session scrolling.
- Selected-card and selected-option UI state.
- Auto-open policy that links selected workspace sessions to pending approvals.
- CSS for the approval overlay, cards, details, and options.

## Proposed Component Tree

```text
AgentApprovalsWindow
  AgentApprovalsOverlay
    ApprovalCarousel
      EmptyApprovalCard
      ApprovalCard*
        ApprovalHeader
          ProjectIcon
          ProjectPathLabel
          AgentNameLabel
          ModelLabel
        PromptLabel
        DetailPane
          DiffDetailView | CommandDetailView | TextDetailView
        ApprovalOptionList
          ApprovalOptionButton*
    CarouselDots
```

`AgentApprovalsWindow` should be the layer-shell surface:

- Layer: overlay.
- Anchors: top, bottom, left, right.
- Exclusive zone: normal/no reserved space.
- Keyboard mode: exclusive while visible.
- Namespace/name: `agent-approvals`.
- Monitor: active monitor, matching the existing overlay behavior.

`AgentApprovalsOverlay` should hold interaction state. `ApprovalCard` can be a
row/child component keyed by a stable approval id.

## Initial Models

Start with plain Rust data structs for D-Bus data and derived view data, then
wrap provider-backed fields in `#[shell_macros::model]` models as macro support
allows.

```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentStatus {
    pub agent_name: String,
    pub is_subagent: bool,
    pub parent_session_id: String,
    pub agent_nickname: String,
    pub agent_role: String,
    pub state: AgentState,
    pub task_complete: bool,
    pub requires_attention: bool,
    pub attention_reasons: Vec<String>,
    pub context_pct: f64,
    pub model_name: String,
    pub cwd: String,
    pub cost_usd: f64,
    pub pending_prompt: String,
    pub pending_detail_kind: String,
    pub pending_detail_text: String,
    pub pending_options: Vec<String>,
    pub pending_option_descriptions: Vec<String>,
    pub pending_count: u32,
    pub pending_request_ids: Vec<String>,
    pub pending_prompts: Vec<String>,
    pub pending_detail_kinds: Vec<String>,
    pub pending_detail_texts: Vec<String>,
    pub pending_options_list: Vec<Vec<String>>,
    pub pending_option_descriptions_list: Vec<Vec<String>>,
    pub session_name: String,
    pub five_hour_usage_pct: f64,
    pub five_hour_resets_at: i64,
    pub seven_day_usage_pct: f64,
    pub seven_day_resets_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AgentState {
    #[default]
    NoSession,
    Idle,
    Thinking,
    ToolUse,
    Compacting,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingApproval {
    pub session_id: String,
    pub request_id: String,
    pub prompt: String,
    pub detail_kind: ApprovalDetailKind,
    pub detail_text: String,
    pub options: Vec<ApprovalOption>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalOption {
    pub label: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ApprovalDetailKind {
    Diff,
    Command,
    Json,
    #[default]
    Text,
    Other(String),
}

#[shell_macros::model]
pub struct AgentApprovalsModel {
    #[source(agent_sessions_provider())]
    pub sessions: std::collections::BTreeMap<String, AgentStatus>,

    #[source(selected_workspace_agent_sessions_provider())]
    pub selected_workspace_session_nodes: Vec<String>,

    pub visible: bool,
    pub target_session_id: Option<String>,
    pub pending: Vec<PendingApproval>,
    pub selected_card: usize,
    pub selected_option: usize,
    pub last_auto_open_key: Option<String>,
}

#[shell_macros::model]
pub struct ApprovalCardModel {
    pub approval: PendingApproval,
    pub cwd_display: String,
    pub project_icon: String,
    pub agent_name_display: String,
    pub model_name: String,
}
```

The first implementation can keep `pending` as a derived field updated whenever
`sessions` changes. Longer term, a derived provider should emit
`Vec<PendingApproval>` directly so view models remain simple.

## Provider And Stream Dependencies

Required provider sources:

- `AgentSessionsProvider`: session-bus ObjectManager provider for
  `io.github.AgentDBus` at `/io/github/AgentDBus`, tracking session objects under
  `/io/github/AgentDBus/sessions/`.
- Agent session property stream: initial `GetManagedObjects`, then
  `InterfacesAdded`, `InterfacesRemoved`, and `PropertiesChanged` for
  `io.github.AgentDBus1.Session`.
- Agent response command: method call on the session object,
  `RespondToElicitationById(request_id, answer)` when request id is present,
  otherwise `RespondToElicitation(answer)`.
- Selected workspace provider from Locus graph.
- Selected workspace source window list from Locus graph.
- Window-to-agent-session target list from Locus graph relation
  `window-agent-session`.
- Project icon provider for each session node, using workspace-project icon first
  and direct project icon second, with `smart_toy` fallback.
- Local visibility command stream for manual show/hide/toggle/show-for-session.

Useful derived streams:

- `pending_approvals = derive_pending_approvals(sessions)`.
- `pending_approval_signatures` for cheap equality and row diffing.
- `workspace_pending_match = first pending approval whose
  agent-session:{session_id} is linked from a selected-workspace window`.
- `auto_open_key = {session_id}:{prompt}`.

The old standalone elicitation signal stream is not sufficient for this overlay
because the UI is driven by durable pending session properties. It may still be
useful for other immediate notification surfaces, but approvals should derive
from `AgentStatus`.

## Pending Approval Derivation

Derive pending approvals with this policy:

1. Ignore sessions where `requires_attention` is false.
2. If `pending_request_ids` is non-empty, create one approval per request id.
3. For indexed approvals, read prompt, detail kind, detail text, options, and
   option descriptions from the corresponding indexed arrays.
4. Fall back from missing indexed values to the session-level pending values.
5. Drop indexed approvals with an empty prompt.
6. If there are no request ids, create one approval from session-level pending
   fields when `pending_prompt` is non-empty.
7. If options are empty, display `Allow` and `Deny`.
8. If option descriptions are missing, use empty descriptions.

Use a stable approval key such as `{session_id}:{request_id}`. For legacy
single-request sessions with no request id, include the prompt in the key to
avoid collapsing changed requests from the same session.

## Auto-Open Derivation

Auto-open should be derived from provider state, not from widget internals:

1. Resolve the selected workspace.
2. Resolve source windows for that workspace.
3. Resolve every `window-agent-session` target for those windows.
4. Convert linked graph nodes to session ids by matching
   `agent-session:{session_id}`.
5. Find the first pending approval whose session is linked to the selected
   workspace.
6. Open the overlay for that session when `{session_id}:{prompt}` differs from
   the last auto-open key.

Manual open/toggle should not overwrite `last_auto_open_key`; only successful
auto-open should. Answering or clearing an approval should not force-close the
overlay if other pending approvals remain.

## Styling Notes

Port `style/agent-approvals.scss` into the consumer crate stylesheet with the
same class names initially:

- `agent-approval-window`
- `agent-approval-body`
- `agent-approval-card`
- `agent-approval-header`
- `agent-approval-project`
- `agent-approval-agent`
- `agent-approval-model`
- `agent-approval-prompt`
- `agent-approval-content-scroll`
- `agent-approval-detail*`
- `agent-approval-option*`
- `agent-approval-key`
- `agent-approval-empty*`

Do not move this styling into `shell/core`; consumer CSS owns the visual design.

## Missing Shell Features

- [D-Bus ObjectManager collection provider](../../missing-shell-features/dbus-object-manager-provider.md):
  needed to bootstrap, add, remove, and update dynamic agent session objects.
- [D-Bus method command provider](../../missing-shell-features/dbus-method-commands.md):
  needed for `RespondToElicitation` and `RespondToElicitationById`.
- [Derived provider combinators](../../missing-shell-features/derived-provider-combinators.md):
  needed for pending approval derivation, auto-open matching, distinct
  signatures, and project icon fallback.
- [Locus graph collection providers](../../missing-shell-features/locus-graph-collection-providers.md):
  needed for selected workspace windows and window-to-agent-session targets.
- [Dynamic child lists](../../missing-shell-features/dynamic-child-lists.md):
  needed for carousel cards and option rows keyed by approval data.
- [Layer-shell keyboard mode binding](../../missing-shell-features/layer-shell-keyboard-mode.md):
  needed to request exclusive keyboard input while the overlay is visible.
- [Keyboard shortcuts](../../missing-shell-features/keyboard-shortcuts.md):
  needed for dismissal, card navigation, and option selection.
- [Carousel or paged collection widget support](../../missing-shell-features/carousel-widget.md):
  needed to preserve horizontal card paging and indicator dots.
- [GtkSourceView integration](../../missing-shell-features/gtksourceview-integration.md):
  needed for highlighted diff and command detail panes.
- [CLI request bridge](../../missing-shell-features/cli-request-bridge.md):
  needed if `agent-approvals` must remain openable through the existing AGS
  request-command mechanism.

## Open Questions

- Should the Agent D-Bus contract live only in `rsynapse-shell`, or should a
  feature-gated `common-providers` module expose typed definitions for reuse?
- Should old elicitation signals be consumed anywhere after approvals move to
  durable pending session properties?
- Should the overlay keep the current full-screen scrim, or should the migrated
  shell introduce a shared modal overlay pattern for all full-screen surfaces?
- Can project icon fallback be represented by generated Locus schema helpers, or
  does it need a custom derived provider in the consumer crate?
