//! `ui.*` channels: readiness and render errors of the React app, plus the readiness watchdog.

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;

/// `ui.ready` must arrive this many seconds after start, or the run is reported as broken.
pub const READY_TIMEOUT_SECS: f32 = 15.0;

/// React → Bevy message `ui.ready`: the React app mounted (again after a full hot reload).
#[react_message(name = "ui.ready")]
pub struct UiReady;

/// React → Bevy message `ui.error`: the top-level ErrorBoundary caught a render error.
#[react_message(name = "ui.error")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiError {
    /// Error message.
    pub message: String,
    /// Component stack, if React provided one.
    #[ts(optional)]
    pub stack: Option<String>,
}

/// Health of the React UI; automation runs exit non-zero when it is not ready or has errors.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct UiStatus {
    /// `ui.ready` arrived.
    pub ready: bool,
    /// `ui.error` messages received.
    pub errors: u32,
}

pub(crate) fn register(app: &mut App) {
    app.add_react_handler(on_ready).add_react_handler(on_error);
}

fn on_ready(_: On<UiReady>, status: Option<ResMut<UiStatus>>) {
    if let Some(mut status) = status {
        if !status.ready {
            info!("UI ready");
        }
        status.ready = true;
    }
}

fn on_error(error: On<UiError>, status: Option<ResMut<UiStatus>>) {
    let error = error.event();
    error!(
        "UI error: {}\n{}",
        error.message,
        error.stack.as_deref().unwrap_or("")
    );
    if let Some(mut status) = status {
        status.errors += 1;
    }
}

/// Logs an error (and ends automated runs with a failure) when `ui.ready` is late.
pub(crate) fn watchdog(
    time: Res<Time<Real>>,
    status: Res<UiStatus>,
    automation: Option<Res<AutomationConfig>>,
    mut reported: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    if status.ready || *reported || time.elapsed_secs() < READY_TIMEOUT_SECS {
        return;
    }
    *reported = true;
    error!(
        "ui.ready did not arrive within {READY_TIMEOUT_SECS} s (UI bundle missing, threw, or not mounted)"
    );
    if automation.is_some_and(|a| a.is_automated()) {
        exit.write(AppExit::error());
    }
}
