use crate::application::SessionData;
use crate::core::{when_sync, when_sync_with_item};
use crate::domain::{MidiControlInput, MidiFeedbackOutput, Session};
use crate::domain::{ReaperTarget, SharedSession};
use crate::infrastructure::common::bindings::root;
use crate::infrastructure::ui::MainPanel;
use c_str_macro::c_str;
use helgoboss_midi::Channel;
use reaper_high::{MidiInputDevice, MidiOutputDevice, Reaper};
use reaper_low::Swell;
use reaper_medium::{MidiInputDeviceId, MidiOutputDeviceId, ReaperString};
use rx_util::{LocalProp, UnitEvent};
use rxrust::prelude::*;
use std::cell::{Cell, Ref, RefCell};
use std::ffi::CString;
use std::iter;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::time::Duration;
use swell_ui::{Clipboard, SharedView, View, ViewContext, Window};

/// The upper part of the main panel, containing buttons such as "Add mapping".
pub struct HeaderPanel {
    view: ViewContext,
    session: SharedSession,
    main_panel: SharedView<MainPanel>,
}

impl HeaderPanel {
    pub fn new(session: SharedSession, main_panel: SharedView<MainPanel>) -> HeaderPanel {
        HeaderPanel {
            view: Default::default(),
            session,
            main_panel,
        }
    }
}

impl HeaderPanel {
    fn toggle_learn_source_filter(&self) {
        // TODO-high
    }

    fn toggle_learn_target_filter(&self) {
        let mut learning = self.main_panel.is_learning_target_filter.borrow_mut();
        if learning.get() {
            // Stop learning
            learning.set(false);
        } else {
            // Start learning
            learning.set(true);
            when_sync_with_item(
                ReaperTarget::touched()
                    .take_until(learning.changed_to(false))
                    .take(1),
                self.view.closed(),
                &self.main_panel,
                |main_panel, target| {
                    main_panel
                        .target_filter
                        .borrow_mut()
                        .set(Some((*target).clone()));
                },
                |main_panel| {
                    main_panel.is_learning_target_filter.borrow_mut().set(false);
                },
            );
        }
    }

    fn clear_source_filter(&self) {
        // TODO-high
    }

    fn clear_target_filter(&self) {
        self.main_panel.target_filter.borrow_mut().set(None);
    }

    fn update_let_matched_events_through(&self) {
        self.session.borrow_mut().let_matched_events_through.set(
            self.view
                .require_control(root::ID_LET_MATCHED_EVENTS_THROUGH_CHECK_BOX)
                .is_checked(),
        );
    }

    fn update_let_unmatched_events_through(&self) {
        self.session.borrow_mut().let_unmatched_events_through.set(
            self.view
                .require_control(root::ID_LET_UNMATCHED_EVENTS_THROUGH_CHECK_BOX)
                .is_checked(),
        );
    }

    fn update_send_feedback_only_if_armed(&self) {
        self.session.borrow_mut().send_feedback_only_if_armed.set(
            self.view
                .require_control(root::ID_SEND_FEEDBACK_ONLY_IF_ARMED_CHECK_BOX)
                .is_checked(),
        );
    }

    fn update_always_auto_detect(&self) {
        self.session.borrow_mut().always_auto_detect.set(
            self.view
                .require_control(root::ID_ALWAYS_AUTO_DETECT_MODE_CHECK_BOX)
                .is_checked(),
        );
    }

    fn invalidate_all_controls(&self) {
        self.invalidate_midi_control_input_combo_box();
        self.invalidate_midi_feedback_output_combo_box();
        self.invalidate_let_matched_events_through_check_box();
        self.invalidate_let_unmatched_events_through_check_box();
        self.invalidate_send_feedback_only_if_armed_check_box();
        self.invalidate_always_auto_detect_check_box();
        self.invalidate_source_filter_buttons();
        self.invalidate_target_filter_buttons();
    }

    fn invalidate_midi_control_input_combo_box(&self) {
        self.invalidate_midi_control_input_combo_box_options();
        self.invalidate_midi_control_input_combo_box_value();
    }

    fn invalidate_midi_control_input_combo_box_options(&self) {
        let b = self.view.require_control(root::ID_CONTROL_DEVICE_COMBO_BOX);
        b.fill_combo_box_with_data_small(
            iter::once((
                -1isize,
                "<FX input> (no support for MIDI clock sources)".to_string(),
            ))
            .chain(
                Reaper::get()
                    .midi_input_devices()
                    .map(|dev| (dev.id().get() as isize, get_midi_input_device_label(dev))),
            ),
        )
    }

    fn invalidate_midi_control_input_combo_box_value(&self) {
        let b = self.view.require_control(root::ID_CONTROL_DEVICE_COMBO_BOX);
        use MidiControlInput::*;
        match self.session.borrow().midi_control_input.get() {
            FxInput => {
                b.select_combo_box_item_by_data(-1);
            }
            Device(dev) => b
                .select_combo_box_item_by_data(dev.id().get() as _)
                .unwrap_or_else(|_| {
                    b.select_new_combo_box_item(format!("{}. <Unknown>", dev.id().get()));
                }),
        };
    }

    fn invalidate_midi_feedback_output_combo_box(&self) {
        self.invalidate_midi_feedback_output_combo_box_options();
        self.invalidate_midi_feedback_output_combo_box_value();
    }

    fn invalidate_midi_feedback_output_combo_box_options(&self) {
        let b = self
            .view
            .require_control(root::ID_FEEDBACK_DEVICE_COMBO_BOX);
        b.fill_combo_box_with_data_small(
            iter::once((-1isize, "<None>".to_string())).chain(
                Reaper::get()
                    .midi_output_devices()
                    .map(|dev| (dev.id().get() as isize, get_midi_output_device_label(dev))),
            ),
        )
    }

    fn invalidate_midi_feedback_output_combo_box_value(&self) {
        let b = self
            .view
            .require_control(root::ID_FEEDBACK_DEVICE_COMBO_BOX);
        use MidiFeedbackOutput::*;
        match self.session.borrow().midi_feedback_output.get() {
            None => {
                b.select_combo_box_item_by_data(-1);
            }
            Some(o) => match o {
                FxOutput => todo!("feedback to FX output not yet supported"),
                Device(dev) => b
                    .select_combo_box_item_by_data(dev.id().get() as _)
                    .unwrap_or_else(|_| {
                        b.select_new_combo_box_item(format!("{}. <Unknown>", dev.id().get()));
                    }),
            },
        };
    }

    fn update_midi_control_input(&self) {
        let b = self.view.require_control(root::ID_CONTROL_DEVICE_COMBO_BOX);
        let value = match b.selected_combo_box_item_data() {
            -1 => MidiControlInput::FxInput,
            id if id >= 0 => {
                let dev = Reaper::get().midi_input_device_by_id(MidiInputDeviceId::new(id as _));
                MidiControlInput::Device(dev)
            }
            _ => unreachable!(),
        };
        self.session.borrow_mut().midi_control_input.set(value);
    }

    fn update_midi_feedback_output(&self) {
        let b = self
            .view
            .require_control(root::ID_FEEDBACK_DEVICE_COMBO_BOX);
        let value = match b.selected_combo_box_item_data() {
            -1 => None,
            id if id >= 0 => {
                let dev = Reaper::get().midi_output_device_by_id(MidiOutputDeviceId::new(id as _));
                Some(MidiFeedbackOutput::Device(dev))
            }
            _ => todo!("feedback to FX output not yet supported"),
        };
        self.session.borrow_mut().midi_feedback_output.set(value);
    }

    fn invalidate_let_matched_events_through_check_box(&self) {
        let b = self
            .view
            .require_control(root::ID_LET_MATCHED_EVENTS_THROUGH_CHECK_BOX);
        if self.session.borrow().midi_control_input.get() == MidiControlInput::FxInput {
            b.enable();
            b.set_checked(self.session.borrow().let_matched_events_through.get());
        } else {
            b.disable();
            b.uncheck();
        }
    }

    fn invalidate_let_unmatched_events_through_check_box(&self) {
        let b = self
            .view
            .require_control(root::ID_LET_UNMATCHED_EVENTS_THROUGH_CHECK_BOX);
        if self.session.borrow().midi_control_input.get() == MidiControlInput::FxInput {
            b.enable();
            b.set_checked(self.session.borrow().let_unmatched_events_through.get());
        } else {
            b.disable();
            b.uncheck();
        }
    }

    fn invalidate_send_feedback_only_if_armed_check_box(&self) {
        let b = self
            .view
            .require_control(root::ID_SEND_FEEDBACK_ONLY_IF_ARMED_CHECK_BOX);
        if self.session.borrow().containing_fx_is_in_input_fx_chain() {
            b.disable();
            b.check();
        } else {
            b.enable();
            b.set_checked(self.session.borrow().send_feedback_only_if_armed.get());
        }
    }

    fn invalidate_always_auto_detect_check_box(&self) {
        self.view
            .require_control(root::ID_ALWAYS_AUTO_DETECT_MODE_CHECK_BOX)
            .set_checked(self.session.borrow().always_auto_detect.get());
    }

    fn invalidate_source_filter_buttons(&self) {
        // TODO
    }

    fn invalidate_target_filter_buttons(&self) {
        let learn_button_text = if self.main_panel.is_learning_target_filter.borrow().get() {
            "Stop"
        } else {
            "Learn target filter"
        };
        self.view
            .require_control(root::ID_FILTER_BY_TARGET_BUTTON)
            .set_text(learn_button_text);
        let clear_button_enabled = self.main_panel.target_filter.borrow().get_ref().is_some();
        self.view
            .require_control(root::ID_CLEAR_TARGET_FILTER_BUTTON)
            .set_enabled(clear_button_enabled);
    }

    pub fn import_from_clipboard(&self) {
        let clipboard = Clipboard::new();
        let json = clipboard.read_text().expect("couldn't read from clipboard");
        let session_data: SessionData =
            serde_json::from_str(json.as_str()).expect("invalid session data");
        session_data.apply_to_model(self.session.borrow_mut().deref_mut());
    }

    pub fn export_to_clipboard(&self) {
        let session_data = SessionData::from_model(self.session.borrow().deref());
        let json =
            serde_json::to_string_pretty(&session_data).expect("couldn't serialize session data");
        let clipboard = Clipboard::new();
        clipboard.write_text(json.as_str());
    }

    fn register_listeners(self: SharedView<Self>) {
        let session = self.session.borrow();
        self.when(session.let_matched_events_through.changed(), |view| {
            view.invalidate_let_matched_events_through_check_box()
        });
        self.when(session.let_unmatched_events_through.changed(), |view| {
            view.invalidate_let_unmatched_events_through_check_box()
        });
        self.when(session.send_feedback_only_if_armed.changed(), |view| {
            view.invalidate_send_feedback_only_if_armed_check_box()
        });
        self.when(session.always_auto_detect.changed(), |view| {
            view.invalidate_always_auto_detect_check_box()
        });
        self.when(session.midi_control_input.changed(), |view| {
            view.invalidate_midi_control_input_combo_box();
            view.invalidate_let_matched_events_through_check_box();
            view.invalidate_let_unmatched_events_through_check_box();
            let mut session = view.session.borrow_mut();
            if session.always_auto_detect.get() {
                let control_input = session.midi_control_input.get();
                session
                    .send_feedback_only_if_armed
                    .set(control_input != MidiControlInput::FxInput)
            }
        });
        self.when(session.midi_feedback_output.changed(), |view| {
            view.invalidate_midi_feedback_output_combo_box()
        });
        self.when(
            self.main_panel
                .is_learning_target_filter
                .borrow()
                .changed()
                .merge(self.main_panel.target_filter.borrow().changed()),
            |view| {
                view.invalidate_target_filter_buttons();
            },
        );
        // TODO sourceFilterListening, targetFilterListening,
    }

    fn when(
        self: &SharedView<Self>,
        event: impl UnitEvent,
        reaction: impl Fn(SharedView<Self>) + 'static + Copy,
    ) {
        when_sync(event, self.view.closed(), &self, reaction);
    }
}

impl View for HeaderPanel {
    fn dialog_resource_id(&self) -> u32 {
        root::ID_MAPPINGS_DIALOG
    }

    fn view_context(&self) -> &ViewContext {
        &self.view
    }

    fn opened(self: SharedView<Self>, window: Window) -> bool {
        self.invalidate_all_controls();
        self.register_listeners();
        true
    }

    fn button_clicked(self: SharedView<Self>, resource_id: u32) {
        use root::*;
        match resource_id {
            ID_ADD_MAPPING_BUTTON => self.session.borrow_mut().add_default_mapping(),
            ID_FILTER_BY_SOURCE_BUTTON => self.toggle_learn_source_filter(),
            ID_FILTER_BY_TARGET_BUTTON => self.toggle_learn_target_filter(),
            ID_CLEAR_SOURCE_FILTER_BUTTON => self.clear_source_filter(),
            ID_CLEAR_TARGET_FILTER_BUTTON => self.clear_target_filter(),
            ID_IMPORT_BUTTON => self.import_from_clipboard(),
            ID_EXPORT_BUTTON => self.export_to_clipboard(),
            ID_SEND_FEEDBACK_BUTTON => self.session.borrow().send_feedback(),
            ID_LET_MATCHED_EVENTS_THROUGH_CHECK_BOX => self.update_let_matched_events_through(),
            ID_LET_UNMATCHED_EVENTS_THROUGH_CHECK_BOX => self.update_let_unmatched_events_through(),
            ID_SEND_FEEDBACK_ONLY_IF_ARMED_CHECK_BOX => self.update_send_feedback_only_if_armed(),
            ID_ALWAYS_AUTO_DETECT_MODE_CHECK_BOX => self.update_always_auto_detect(),
            _ => {}
        }
    }

    fn option_selected(self: SharedView<Self>, resource_id: u32) {
        use root::*;
        match resource_id {
            ID_CONTROL_DEVICE_COMBO_BOX => self.update_midi_control_input(),
            ID_FEEDBACK_DEVICE_COMBO_BOX => self.update_midi_feedback_output(),
            _ => unreachable!(),
        }
    }
}

fn get_midi_input_device_label(dev: MidiInputDevice) -> String {
    get_midi_device_label(dev.name(), dev.id().get(), dev.is_connected())
}

fn get_midi_output_device_label(dev: MidiOutputDevice) -> String {
    get_midi_device_label(dev.name(), dev.id().get(), dev.is_connected())
}

fn get_midi_device_label(name: ReaperString, raw_id: u8, connected: bool) -> String {
    format!(
        "{}. {}{}",
        raw_id,
        name.to_str(),
        if connected { "" } else { " <not present>" }
    )
}
