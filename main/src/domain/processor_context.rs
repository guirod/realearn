use crate::domain::{ControlContext, PluginParams};
use derivative::Derivative;
use reaper_high::{Fx, FxChain, FxChainContext, Project, Reaper, Track};
use reaper_medium::{ParamId, TypeSpecificPluginContext};
use std::ptr::NonNull;
use vst::host::Host;
use vst::plugin::HostCallback;

#[derive(Copy, Clone, Debug)]
pub struct ExtendedProcessorContext<'a> {
    pub context: &'a ProcessorContext,
    pub params: &'a PluginParams,
    pub control_context: ControlContext<'a>,
}

impl<'a> ExtendedProcessorContext<'a> {
    pub fn new(
        context: &'a ProcessorContext,
        params: &'a PluginParams,
        control_context: ControlContext<'a>,
    ) -> Self {
        Self {
            context,
            params,
            control_context,
        }
    }

    pub fn context(&self) -> &'a ProcessorContext {
        self.context
    }

    pub fn params(&self) -> &'a PluginParams {
        self.params
    }

    pub fn control_context(&self) -> ControlContext {
        self.control_context
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct ProcessorContext {
    #[derivative(Debug = "ignore")]
    host: HostCallback,
    containing_fx: Fx,
    project: Option<Project>,
    bypass_param_index: u32,
}

pub const WAITING_FOR_SESSION_PARAM_NAME: &str = "realearn/waiting-for-session";
pub const HELGOBOX_INSTANCE_ID: &str = "instance_id";

impl ProcessorContext {
    pub fn from_host(host: HostCallback) -> Result<ProcessorContext, &'static str> {
        let fx = get_containing_fx(&host)?;
        let project = fx.project();
        let bypass_param = fx
            .parameter_by_id(ParamId::Bypass)
            .ok_or("bypass parameter not found")?;
        let context = ProcessorContext {
            host,
            containing_fx: fx,
            project,
            bypass_param_index: bypass_param.index(),
        };
        Ok(context)
    }

    pub fn containing_fx(&self) -> &Fx {
        &self.containing_fx
    }

    pub fn track(&self) -> Option<&Track> {
        self.containing_fx.track()
    }

    /// Returns the index of ReaLearn's "Bypass" parameter.
    pub fn bypass_param_index(&self) -> u32 {
        self.bypass_param_index
    }

    /// This falls back to the current project if on the monitoring FX chain.
    pub fn project_or_current_project(&self) -> Project {
        self.project
            .unwrap_or_else(|| Reaper::get().current_project())
    }

    pub fn project(&self) -> Option<Project> {
        self.project
    }

    pub fn is_on_monitoring_fx_chain(&self) -> bool {
        matches!(
            self.containing_fx.chain().context(),
            FxChainContext::Monitoring
        )
    }

    pub fn notify_dirty(&self) {
        self.host.automate(-1, 0.0);
    }
}

/// Calling this in the `new()` method is too early. The containing FX can't generally be found
/// when we just open a REAPER project. We must wait for `init()` to be called.
fn get_containing_fx(host: &HostCallback) -> Result<Fx, &'static str> {
    let reaper = Reaper::get();
    let aeffect = NonNull::new(host.raw_effect()).expect("must not be null");
    let plugin_context = reaper.medium_reaper().plugin_context();
    let vst_context = match plugin_context.type_specific() {
        TypeSpecificPluginContext::Vst(ctx) => ctx,
        _ => unreachable!(),
    };
    let fx = if let Some(track) = unsafe { vst_context.request_containing_track(aeffect) } {
        let project = unsafe { vst_context.request_containing_project(aeffect) };
        let track = Track::new(track, Some(project));
        // We could use the following but it only works for REAPER v6.11+, so let's rely on our own
        // technique for now.
        // let location = unsafe { vst_context.request_containing_fx_location(aeffect) };
        find_realearn_fx_waiting_for_session(&track.normal_fx_chain())
            .or_else(|| find_realearn_fx_waiting_for_session(&track.input_fx_chain()))
            .ok_or("couldn't find containing FX on track FX chains")?
    } else if let Some(_take) = unsafe { vst_context.request_containing_take(aeffect) } {
        return Err("ReaLearn as take FX is not supported yet");
    } else {
        find_realearn_fx_waiting_for_session(&reaper.monitoring_fx_chain())
            .ok_or("Couldn't find containing FX on monitoring FX chain. It's okay if this occurs during plug-in scanning.")?
    };
    Ok(fx)
}

fn is_realearn_waiting_for_session(fx: &Fx) -> bool {
    let result = fx.get_named_config_param(WAITING_FOR_SESSION_PARAM_NAME, 1);
    match result {
        Ok(buffer) => *buffer.first().expect("impossible") == 1,
        Err(_) => false,
    }
}

fn find_realearn_fx_waiting_for_session(fx_chain: &FxChain) -> Option<Fx> {
    // TODO-low Use REAPER 6.11 API addition instead, if available
    fx_chain.fxs().find(is_realearn_waiting_for_session)
}
