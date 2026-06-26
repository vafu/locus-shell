use std::{cmp::Ordering, fs};

use shell_core::{
    locus_path::LocusPath,
    source::{self, NodeState, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use crate::locusfs_paths::DBUS_SESSION;

use super::view::{self, BzBusView, Invocation};

const BZBUS_MANAGER_PATH: &str = "/com/snap/BzBus";
const BZBUS_INVOCATIONS_PATH: &str = "/com/snap/BzBus/invocations";

#[derive(Clone, Debug, Eq, PartialEq)]
struct InvocationCore {
    id: String,
    build_id: String,
    component: String,
    source: String,
    last_sequence: i64,
    status: String,
    outcome: String,
    command_name: String,
    started_at_unix_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InvocationProgress {
    ended_at_unix_ms: i64,
    progress_completed: u32,
    progress_total: u32,
    actions_completed: u32,
    total_actions: u64,
    actions_failed: u32,
    running_actions: u32,
}

pub(in crate::widgets::bar) fn bzbus_status() -> Observable<BzBusView> {
    source::shared_by_key("rsynapse.bzbus-status", "all", || {
        let active = DBUS_SESSION
            .object(BZBUS_MANAGER_PATH)
            .as_node()
            .map(|state| state == NodeState::Present);
        let invocation = selected_invocation().switch_map(|path| match path {
            Some(path) => invocation(path).map(|invocation| vec![invocation]).box_it(),
            None => source::once(Vec::new()),
        });

        combine_latest!(active, invocation => |(active, invocations)| view::view(active, invocations))
            .distinct_until_changed()
            .box_it()
    })
}

fn selected_invocation() -> Observable<Option<LocusPath>> {
    let invocations = DBUS_SESSION.object(BZBUS_INVOCATIONS_PATH);
    let initial = select_invocation(&invocations);
    invocations
        .as_children_events()
        .map(move |_| select_invocation(&invocations))
        .start_with(vec![initial])
        .distinct_until_changed()
        .box_it()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InvocationSelection {
    path: LocusPath,
    id: String,
    last_sequence: i64,
    status: String,
    outcome: String,
    started_at_unix_ms: i64,
    ended_at_unix_ms: i64,
}

fn select_invocation(invocations: &LocusPath) -> Option<LocusPath> {
    fs::read_dir(invocations.as_path())
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| LocusPath::new(entry.path()))
        .filter_map(selection_snapshot)
        .max_by(compare_selections)
        .map(|selection| selection.path)
}

fn selection_snapshot(path: LocusPath) -> Option<InvocationSelection> {
    path.as_path().is_dir().then(|| InvocationSelection {
        id: read_prop(&path, "Id"),
        last_sequence: parse_i64(&read_prop(&path, "LastObservedSequenceNumber")),
        status: read_prop(&path, "Status"),
        outcome: read_prop(&path, "Outcome"),
        started_at_unix_ms: parse_i64(&read_prop(&path, "StartedAtUnixMs")),
        ended_at_unix_ms: parse_i64(&read_prop(&path, "EndedAtUnixMs")),
        path,
    })
}

fn compare_selections(left: &InvocationSelection, right: &InvocationSelection) -> Ordering {
    selection_is_active(left)
        .cmp(&selection_is_active(right))
        .then_with(|| selection_observed_time(left).cmp(&selection_observed_time(right)))
        .then_with(|| left.last_sequence.cmp(&right.last_sequence))
        .then_with(|| left.id.cmp(&right.id))
}

fn selection_is_active(invocation: &InvocationSelection) -> bool {
    invocation.ended_at_unix_ms <= 0
        && !matches!(
            normalized(invocation.status.as_str()).as_str(),
            "finished" | "failed" | "success"
        )
        && !matches!(
            normalized(invocation.outcome.as_str()).as_str(),
            "finished" | "failed" | "success"
        )
}

fn selection_observed_time(invocation: &InvocationSelection) -> i64 {
    invocation
        .ended_at_unix_ms
        .max(invocation.started_at_unix_ms)
}

fn read_prop(path: &LocusPath, property: &str) -> String {
    fs::read_to_string(path.prop(property).as_path())
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

fn invocation(path: LocusPath) -> Observable<Invocation> {
    let core = combine_latest!(
        path.observe_prop_or::<String>("Id", String::new()),
        path.observe_prop_or::<String>("BuildId", String::new()),
        path.observe_prop_or::<String>("Component", String::new()),
        path.observe_prop_or::<String>("Source", String::new()),
        path.observe_prop_or::<String>("LastObservedSequenceNumber", String::new())
            .map(|value| parse_i64(&value)),
        path.observe_prop_or::<String>("Status", String::new()),
        path.observe_prop_or::<String>("Outcome", String::new()),
        path.observe_prop_or::<String>("CommandName", String::new()),
        path.observe_prop_or::<String>("StartedAtUnixMs", String::new())
            .map(|value| parse_i64(&value))
            => |(id, build_id, component, source, last_sequence, status, outcome, command_name, started_at_unix_ms)| {
                InvocationCore {
                    id,
                    build_id,
                    component,
                    source,
                    last_sequence,
                    status,
                    outcome,
                    command_name,
                    started_at_unix_ms,
                }
            },
    );
    let progress = combine_latest!(
        path.observe_prop_or::<String>("EndedAtUnixMs", String::new())
            .map(|value| parse_i64(&value)),
        path.observe_prop_or::<String>("ProgressCompleted", String::new())
            .map(|value| parse_u32(&value)),
        path.observe_prop_or::<String>("ProgressTotal", String::new())
            .map(|value| parse_u32(&value)),
        path.observe_prop_or::<String>("ActionsCompleted", String::new())
            .map(|value| parse_u32(&value)),
        path.observe_prop_or::<String>("TotalActions", String::new())
            .map(|value| parse_i64(&value))
            .map(nonnegative),
        path.observe_prop_or::<String>("ActionsFailed", String::new())
            .map(|value| parse_u32(&value)),
        path.observe_prop_or::<String>("RunningActions", String::new())
            .map(|value| parse_u32(&value))
            => |(ended_at_unix_ms, progress_completed, progress_total, actions_completed, total_actions, actions_failed, running_actions)| {
                InvocationProgress {
                    ended_at_unix_ms,
                    progress_completed,
                    progress_total,
                    actions_completed,
                    total_actions,
                    actions_failed,
                    running_actions,
                }
            },
    );

    combine_latest!(core, progress => |(core, progress)| Invocation {
        id: core.id,
        build_id: core.build_id,
        component: core.component,
        source: core.source,
        last_sequence: core.last_sequence,
        status: core.status,
        outcome: core.outcome,
        command_name: core.command_name,
        started_at_unix_ms: core.started_at_unix_ms,
        ended_at_unix_ms: progress.ended_at_unix_ms,
        progress_completed: progress.progress_completed,
        progress_total: progress.progress_total,
        actions_completed: progress.actions_completed,
        total_actions: progress.total_actions,
        actions_failed: progress.actions_failed,
        running_actions: progress.running_actions,
    })
    .distinct_until_changed()
    .box_it()
}

pub(super) fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(super) fn parse_i64(value: &str) -> i64 {
    parse_wrapped_number(value)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

pub(super) fn parse_u32(value: &str) -> u32 {
    parse_wrapped_number(value)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn parse_wrapped_number(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    value
        .strip_prefix("OwnedValue(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(value)
        .split_once('(')
        .and_then(|(_, value)| value.strip_suffix(')'))
        .or(Some(value))
}

fn nonnegative(value: i64) -> u64 {
    value.max(0) as u64
}
