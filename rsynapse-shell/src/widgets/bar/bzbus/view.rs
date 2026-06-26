use std::time::{SystemTime, UNIX_EPOCH};

const ACTIVE_STALE_MS: i64 = 2 * 60 * 60 * 1000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BzBusView {
    pub(in crate::widgets::bar) classes: Vec<&'static str>,
    pub(in crate::widgets::bar) tooltip: String,
    pub(in crate::widgets::bar) icon: &'static str,
    pub(in crate::widgets::bar) label: String,
}

impl Default for BzBusView {
    fn default() -> Self {
        Self {
            classes: classes_for(false, None),
            tooltip: "bzbus offline".to_owned(),
            icon: "cloud_off",
            label: "offline".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Invocation {
    pub(super) id: String,
    pub(super) build_id: String,
    pub(super) component: String,
    pub(super) source: String,
    pub(super) last_sequence: i64,
    pub(super) status: String,
    pub(super) outcome: String,
    pub(super) command_name: String,
    pub(super) started_at_unix_ms: i64,
    pub(super) ended_at_unix_ms: i64,
    pub(super) progress_completed: u32,
    pub(super) progress_total: u32,
    pub(super) actions_completed: u32,
    pub(super) total_actions: u64,
    pub(super) actions_failed: u32,
    pub(super) running_actions: u32,
}

pub(super) fn view(active: bool, mut invocations: Vec<Invocation>) -> BzBusView {
    if !active {
        return BzBusView::default();
    }

    invocations.sort_by(compare_invocations);
    let invocation = invocations.first();
    BzBusView {
        classes: classes_for(true, invocation),
        tooltip: tooltip(invocation),
        icon: icon_for(active, invocation),
        label: status_text(invocation),
    }
}

fn compare_invocations(left: &Invocation, right: &Invocation) -> std::cmp::Ordering {
    is_active(right)
        .cmp(&is_active(left))
        .then_with(|| observed_time(right).cmp(&observed_time(left)))
        .then_with(|| right.last_sequence.cmp(&left.last_sequence))
        .then_with(|| right.id.cmp(&left.id))
}

fn is_active(invocation: &Invocation) -> bool {
    !is_ended(invocation)
        && !is_stale(invocation)
        && !matches!(
            normalized(invocation.status.as_str()).as_str(),
            "finished" | "failed"
        )
}

fn is_failed(invocation: &Invocation) -> bool {
    matches!(
        normalized(invocation.status.as_str()).as_str(),
        "failed" | "failure" | "error"
    ) || matches!(
        normalized(invocation.outcome.as_str()).as_str(),
        "failed" | "failure" | "error"
    )
}

fn is_finished(invocation: &Invocation) -> bool {
    matches!(
        normalized(invocation.status.as_str()).as_str(),
        "finished" | "success"
    ) || matches!(
        normalized(invocation.outcome.as_str()).as_str(),
        "finished" | "success"
    )
}

fn is_ended(invocation: &Invocation) -> bool {
    invocation.ended_at_unix_ms > 0 || is_failed(invocation) || is_finished(invocation)
}

fn is_stale(invocation: &Invocation) -> bool {
    let last_observed = observed_time(invocation);
    last_observed > 0 && now_unix_ms() - last_observed > ACTIVE_STALE_MS
}

fn observed_time(invocation: &Invocation) -> i64 {
    invocation
        .ended_at_unix_ms
        .max(invocation.started_at_unix_ms)
}

fn display_status(invocation: &Invocation) -> String {
    if is_failed(invocation) {
        return "failed".to_owned();
    }
    if is_finished(invocation) {
        return "finished".to_owned();
    }
    if invocation.ended_at_unix_ms > 0 {
        return "ended".to_owned();
    }
    if is_stale(invocation) {
        return "stale".to_owned();
    }
    non_empty(invocation.status.as_str())
        .unwrap_or("unknown")
        .to_owned()
}

fn status_text(invocation: Option<&Invocation>) -> String {
    let Some(invocation) = invocation else {
        return "idle".to_owned();
    };

    let mut parts = vec![display_status(invocation), compact_elapsed_text(invocation)];
    if let Some(work) = work_text(invocation) {
        parts.push(work);
    }
    if invocation.actions_failed > 0 {
        parts.push(format!("{}!", invocation.actions_failed));
    }
    parts.join(" · ")
}

fn tooltip(invocation: Option<&Invocation>) -> String {
    let Some(invocation) = invocation else {
        return "bzbus connected · no active build".to_owned();
    };

    let mut lines = vec![
        format!(
            "status: {} ({})",
            display_status(invocation),
            non_empty(invocation.outcome.as_str()).unwrap_or("unknown")
        ),
        format!("elapsed: {}", elapsed_text(invocation)),
        format!("command: {}", command_text(invocation)),
        format!(
            "actions: {} completed, {} total, {} failed, {} running",
            invocation.actions_completed,
            invocation.total_actions,
            invocation.actions_failed,
            invocation.running_actions
        ),
    ];
    if invocation.progress_total > 0 {
        lines.push(format!("progress: {}", progress_text(invocation)));
    }
    if let Some(component) = non_empty(invocation.component.as_str()) {
        lines.push(format!("component: {component}"));
    }
    if let Some(source) = non_empty(invocation.source.as_str()) {
        lines.push(format!("source: {source}"));
    }
    lines.push(format!("sequence: {}", invocation.last_sequence));
    lines.push(format!("invocation: {}", fallback(invocation.id.as_str())));
    lines.push(format!("build: {}", fallback(invocation.build_id.as_str())));
    lines.join("\n")
}

fn icon_for(active: bool, invocation: Option<&Invocation>) -> &'static str {
    let Some(invocation) = invocation else {
        return if active { "construction" } else { "cloud_off" };
    };
    if is_failed(invocation) {
        "error"
    } else if is_finished(invocation) {
        "check_circle"
    } else {
        "build_circle"
    }
}

fn classes_for(active: bool, invocation: Option<&Invocation>) -> Vec<&'static str> {
    let mut classes = vec!["barblock", "bzbus-widget"];
    if !active {
        classes.push("offline");
    } else if let Some(invocation) = invocation {
        if is_failed(invocation) {
            classes.push("failed");
        } else if is_finished(invocation) {
            classes.push("finished");
        } else if is_ended(invocation) || is_stale(invocation) {
            classes.push("idle");
        } else {
            classes.push("running");
        }
    } else {
        classes.push("idle");
    }
    classes
}

fn command_text(invocation: &Invocation) -> &str {
    non_empty(invocation.command_name.as_str()).unwrap_or("unknown")
}

fn work_text(invocation: &Invocation) -> Option<String> {
    if invocation.progress_total > 0 {
        return Some(progress_text(invocation));
    }
    if invocation.total_actions > 0 {
        return Some(format!("{}a", invocation.total_actions));
    }
    (invocation.actions_completed > 0).then(|| format!("{}a", invocation.actions_completed))
}

fn progress_text(invocation: &Invocation) -> String {
    let mut text = format!(
        "{}/{}",
        invocation.progress_completed, invocation.progress_total
    );
    if invocation.actions_completed > 0 {
        text.push_str(format!(" · {}a", invocation.actions_completed).as_str());
    }
    if invocation.running_actions > 0 {
        text.push_str(format!("/{}r", invocation.running_actions).as_str());
    }
    text
}

fn elapsed_text(invocation: &Invocation) -> String {
    if invocation.started_at_unix_ms <= 0 {
        return "unknown".to_owned();
    }
    duration_text(invocation_end(invocation) - invocation.started_at_unix_ms)
}

fn compact_elapsed_text(invocation: &Invocation) -> String {
    if invocation.started_at_unix_ms <= 0 {
        return "unknown".to_owned();
    }
    let total_seconds =
        ((invocation_end(invocation) - invocation.started_at_unix_ms) / 1000).max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        "<1m".to_owned()
    }
}

fn invocation_end(invocation: &Invocation) -> i64 {
    if invocation.ended_at_unix_ms > 0 {
        invocation.ended_at_unix_ms
    } else {
        now_unix_ms()
    }
}

fn duration_text(ms: i64) -> String {
    let total_seconds = (ms / 1000).max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn fallback(value: &str) -> &str {
    non_empty(value).unwrap_or("unknown")
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
