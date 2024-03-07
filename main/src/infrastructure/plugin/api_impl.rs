use crate::infrastructure::plugin::BackboneShell;
use anyhow::Context;
use realearn_api::runtime::{register_helgobox_api, HelgoboxApi};
use reaper_high::{OrCurrentProject, Project, Reaper};
use reaper_low::raw::ReaProject;
use reaper_medium::{ReaperStr, RegistrationObject};
use std::borrow::Cow;
use std::ffi::c_int;

struct HelgoboxAllApi;

impl HelgoboxApi for HelgoboxAllApi {
    extern "C" fn HB_FindFirstHelgoboxInstanceInProject(project: *mut ReaProject) -> c_int {
        find_first_helgobox_instance_in_project(project).unwrap_or(-1)
    }
}

impl playtime_api::runtime::PlaytimeApi for HelgoboxAllApi {
    extern "C" fn HB_FindFirstPlaytimeHelgoboxInstanceInProject(project: *mut ReaProject) -> c_int {
        find_first_playtime_helgobox_instance_in_project(project).unwrap_or(-1)
    }

    extern "C" fn HB_CreateClipMatrix(instance_id: c_int) {
        let _ = create_clip_matrix(instance_id);
    }

    extern "C" fn HB_ShowOrHidePlaytime(instance_id: c_int) {
        let _ = show_or_hide_playtime(instance_id);
    }
}

fn find_first_playtime_helgobox_instance_in_project(
    project: *mut ReaProject,
) -> anyhow::Result<c_int> {
    let project = reaper_medium::ReaProject::new(project)
        .map(Project::new)
        .or_current_project();
    let instance_id = BackboneShell::get()
        .find_first_helgobox_instance_matching(|info| {
            if info.processor_context.project() != Some(project) {
                return false;
            }
            let Some(instance) = info.instance.upgrade() else {
                return false;
            };
            let instance_state = instance.borrow();
            instance_state.has_clip_matrix()
        })
        .context("Project doesn't contain Helgobox instance with a Playtime Clip Matrix")?;
    Ok(u32::from(instance_id) as _)
}

fn find_first_helgobox_instance_in_project(project: *mut ReaProject) -> anyhow::Result<c_int> {
    let project = reaper_medium::ReaProject::new(project)
        .map(Project::new)
        .or_current_project();
    let instance_id = BackboneShell::get()
        .find_first_helgobox_instance_matching(|instance| {
            instance
                .processor_context
                .project()
                .is_some_and(|p| p == project)
        })
        .context("Project doesn't contain Helgobox instance")?;
    Ok(u32::from(instance_id) as _)
}

fn create_clip_matrix(instance_id: c_int) -> anyhow::Result<()> {
    let instance_id = u32::try_from(instance_id)?.into();
    let instance_shell = BackboneShell::get()
        .find_instance_shell_by_instance_id(instance_id)
        .context("instance not found")?;
    instance_shell.insert_owned_clip_matrix_if_necessary()?;
    Ok(())
}

fn show_or_hide_playtime(instance_id: c_int) -> anyhow::Result<()> {
    let instance_id = u32::try_from(instance_id)?;
    let main_panel = BackboneShell::get()
        .find_instance_panel_by_instance_id(instance_id.into())
        .context("Instance not found")?;
    main_panel.start_show_or_hide_app_instance("/playtime".to_string());
    Ok(())
}

pub fn register_api() -> anyhow::Result<()> {
    let mut session = Reaper::get().medium_session();
    register_or_unregister_api(|reg| unsafe {
        session.plugin_register_add(reg)?;
        Ok(())
    })
}

pub fn unregister_api() -> anyhow::Result<()> {
    let mut session = Reaper::get().medium_session();
    register_or_unregister_api(|reg| unsafe {
        session.plugin_register_remove(reg);
        Ok(())
    })
}

fn register_or_unregister_api(
    mut op: impl FnMut(RegistrationObject) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    register_helgobox_api::<HelgoboxAllApi, anyhow::Error>(|name, ptr| unsafe {
        op(RegistrationObject::Api(
            Cow::Borrowed(ReaperStr::from_ptr(name.as_ptr())),
            ptr,
        ))?;
        Ok(())
    })?;
    playtime_api::runtime::register_playtime_api::<HelgoboxAllApi, anyhow::Error>(
        |name, ptr| unsafe {
            op(RegistrationObject::Api(
                Cow::Borrowed(ReaperStr::from_ptr(name.as_ptr())),
                ptr,
            ))?;
            Ok(())
        },
    )?;
    Ok(())
}
