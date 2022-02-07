use crate::buffer::AudioBufMut;
use crate::file_util::get_path_for_new_media_file;
use crate::ClipPlayState::Recording;
use crate::{
    clip_timeline, AudioBuf, AudioSupplier, ClipContent, ClipInfo, ClipRecordInput,
    CreateClipContentMode, ExactDuration, ExactFrameCount, MidiSupplier, OwnedAudioBuffer,
    SourceData, SupplyAudioRequest, SupplyMidiRequest, SupplyResponse, WithFrameRate,
};
use reaper_high::{OwnedSource, Project, Reaper, ReaperSource};
use reaper_low::raw::{
    midi_realtime_write_struct_t, PCM_SINK_EXT_CREATESOURCE, PCM_SOURCE_EXT_ADDMIDIEVENTS,
};
use reaper_low::{raw, PCM_source};
use reaper_medium::{
    BorrowedMidiEventList, DurationInSeconds, Hz, OwnedPcmSink, OwnedPcmSource, PcmSource,
    PositionInSeconds, ReaperString,
};
use std::ffi::CString;
use std::ptr::{null, null_mut, NonNull};
use std::{cmp, mem};

#[derive(Debug)]
pub struct Recorder {
    state: Option<State>,
}

#[derive(Debug)]
enum State {
    Ready(ReadyState),
    Recording(RecordingState),
}

#[derive(Debug)]
struct ReadyState {
    source: OwnedPcmSource,
}

#[derive(Debug)]
struct RecordingState {
    sub_state: RecordingSubState,
    old_source: Option<OwnedPcmSource>,
    project: Option<Project>,
}

#[derive(Debug)]
enum RecordingSubState {
    Audio(RecordingAudioState),
    Midi(RecordingMidiState),
}

#[derive(Debug)]
struct RecordingAudioState {
    sink: OwnedPcmSink,
    temporary_audio_buffer: OwnedAudioBuffer,
    next_record_start_frame: usize,
}

#[derive(Debug)]
struct RecordingMidiState {
    new_source: OwnedPcmSource,
}

impl RecordingSubState {
    fn new(input: ClipRecordInput, project: Option<Project>) -> Self {
        use ClipRecordInput::*;
        match input {
            Midi => Self::Midi(RecordingMidiState {
                new_source: create_empty_midi_source(),
            }),
            Audio => Self::Audio(RecordingAudioState {
                sink: create_audio_sink(project),
                temporary_audio_buffer: OwnedAudioBuffer::new(2, 48000 * 2),
                next_record_start_frame: 0,
            }),
        }
    }
}

#[derive(Copy, Clone)]
pub struct WriteMidiRequest<'a> {
    pub input_sample_rate: Hz,
    pub block_length: usize,
    pub events: &'a BorrowedMidiEventList,
}

#[derive(Copy, Clone)]
pub struct WriteAudioRequest<'a> {
    pub input_sample_rate: Hz,
    pub block_length: usize,
    pub left_buffer: AudioBuf<'a>,
    pub right_buffer: AudioBuf<'a>,
}

impl Recorder {
    pub fn ready(source: OwnedPcmSource) -> Self {
        let ready_state = ReadyState { source };
        Self {
            state: Some(State::Ready(ready_state)),
        }
    }

    pub fn recording(input: ClipRecordInput, project: Option<Project>) -> Self {
        let recording_state = RecordingState {
            sub_state: RecordingSubState::new(input, project),
            old_source: None,
            project,
        };
        Self {
            state: Some(State::Recording(recording_state)),
        }
    }

    pub fn clip_info(&self) -> Option<ClipInfo> {
        let info = match self.state.as_ref().unwrap() {
            State::Ready(s) => {
                ClipInfo {
                    r#type: s.source.get_type(|t| t.to_string()),
                    file_name: s.source.get_file_name(|p| Some(p?.to_owned())),
                    length: {
                        // TODO-low Doesn't need to be optional
                        Some(s.source.duration())
                    },
                }
            }
            // TODO-high Implement last audio moments
            State::Recording(_) => return None,
        };
        Some(info)
    }

    pub fn clip_content(&self, project: Option<Project>) -> Option<ClipContent> {
        let info = match self.state.as_ref().unwrap() {
            State::Ready(s) => {
                let source = ReaperSource::new(s.source.as_ptr());
                let content = ClipContent::from_reaper_source(
                    &source,
                    CreateClipContentMode::AllowEmbeddedData,
                    project,
                );
                content.unwrap()
            }
            // TODO-high Implement last audio moments
            State::Recording(_) => return None,
        };
        Some(info)
    }

    pub fn prepare_recording(&mut self, input: ClipRecordInput, project: Option<Project>) {
        use State::*;
        let old_source = match self.state.take().unwrap() {
            Ready(s) => Some(s.source),
            Recording(s) => s.old_source,
        };
        let recording_state = RecordingState {
            sub_state: RecordingSubState::new(input, project),
            old_source,
            project,
        };
        self.state = Some(Recording(recording_state));
        // TODO-high Just replacing is not a good idea. Fade outs?
    }

    pub fn commit_recording(&mut self) -> Result<SourceData, &'static str> {
        use State::*;
        let (res, next_state) = match self.state.take().unwrap() {
            Ready(s) => (Err("not recording"), Ready(s)),
            Recording(s) => {
                use RecordingSubState::*;
                match s.sub_state {
                    Audio(ss) => {
                        let recording_state = RecordingState {
                            sub_state: Audio(ss),
                            old_source: s.old_source,
                            project: s.project,
                        };
                        (Err("TODO implement"), Recording(recording_state))
                    }
                    Midi(ss) => (
                        Ok(SourceData::from_source(&ss.new_source, s.project)),
                        Ready(ReadyState {
                            source: ss.new_source,
                        }),
                    ),
                }
            }
        };
        self.state = Some(next_state);
        res
    }

    pub fn rollback_recording(&mut self) -> Result<(), &'static str> {
        use State::*;
        let (res, next_state) = match self.state.take().unwrap() {
            Ready(s) => (Ok(()), Ready(s)),
            Recording(s) => {
                if let Some(old_source) = s.old_source {
                    let ready_state = ReadyState { source: old_source };
                    (Ok(()), Ready(ready_state))
                } else {
                    (Err("nothing to roll back to"), Recording(s))
                }
            }
        };
        self.state = Some(next_state);
        res
    }

    pub fn write_audio(&mut self, request: WriteAudioRequest) {
        // // TODO-high Obviously just some experiments.
        let state = match self.state.as_mut().unwrap() {
            State::Recording(RecordingState {
                sub_state: RecordingSubState::Audio(s),
                ..
            }) => s,
            _ => return,
        };
        let start_frame = state.next_record_start_frame;
        let mut out_buf = state.temporary_audio_buffer.to_buf_mut();
        let out_channel_count = out_buf.channel_count();
        let ideal_end_frame = start_frame + request.block_length;
        let end_frame = cmp::min(ideal_end_frame, out_buf.frame_count());
        let num_frames_written = end_frame - start_frame;
        let mut out_buf_slice = out_buf.data_as_mut_slice();
        let left_buf_slice = request.left_buffer.data_as_slice();
        let right_buf_slice = request.right_buffer.data_as_slice();
        for i in 0..num_frames_written {
            out_buf_slice[start_frame * out_channel_count + i * out_channel_count + 0] =
                left_buf_slice[i];
            // out_buf_slice[start_frame + i * out_channel_count + 0] = left_buf_slice[i];
            // out_buf_slice[start_frame + i * out_channel_count + 1] = right_buf_slice[i];
        }
        // request
        //     .left_buffer
        //     .slice(..num_frames_written)
        //     .copy_to(&mut out_buf.slice_mut(start_frame..end_frame));
        state.next_record_start_frame += num_frames_written;
    }

    pub fn write_midi(&mut self, request: WriteMidiRequest, pos: PositionInSeconds) {
        let source = match self.state.as_mut().unwrap() {
            State::Recording(RecordingState {
                sub_state: RecordingSubState::Midi(RecordingMidiState { new_source: source }),
                ..
            })
            | State::Ready(ReadyState { source }) => source,
            _ => return,
        };
        let mut write_struct = midi_realtime_write_struct_t {
            global_time: pos.get(),
            srate: request.input_sample_rate.get(),
            item_playrate: 1.0,
            global_item_time: 0.0,
            length: request.block_length as _,
            // Overdub
            overwritemode: 0,
            events: unsafe { request.events.as_ptr().as_mut() },
            latency: 0.0,
            // Not used
            overwrite_actives: null_mut(),
        };
        unsafe {
            source.extended(
                PCM_SOURCE_EXT_ADDMIDIEVENTS as _,
                &mut write_struct as *mut _ as _,
                null_mut(),
                null_mut(),
            );
        }
    }
}

/// Returns an empty MIDI source prepared for recording.
fn create_empty_midi_source() -> OwnedPcmSource {
    // TODO-high Also implement for audio recording.
    let mut source = OwnedSource::from_type("MIDI").unwrap();
    // TODO-high We absolutely need the permanent section supplier, then we can play the
    //  source correctly positioned and with correct length even the source is too long
    //  and starts too early.
    // The following seems to be the absolute minimum to create the shortest possible MIDI clip
    // (which still is longer than zero).
    let chunk = "\
        HASDATA 1 960 QN\n\
        E 1 b0 7b 00\n\
    >\n\
    ";
    source
        .set_state_chunk("<SOURCE MIDI\n", String::from(chunk))
        .unwrap();
    source.into_raw()
}

fn create_audio_sink(project: Option<Project>) -> OwnedPcmSink {
    let proj_ptr = project.map(|p| p.raw().as_ptr()).unwrap_or(null_mut());
    let file_name = get_path_for_new_media_file("clip-audio", "wav", project);
    let file_name_str = file_name.to_str().unwrap();
    let file_name_c_string = CString::new(file_name_str).unwrap();
    unsafe {
        let sink = Reaper::get().medium_reaper().low().PCM_Sink_CreateEx(
            proj_ptr,
            file_name_c_string.as_ptr(),
            null(),
            0,
            2,
            48000,
            false,
        );
        let sink = NonNull::new(sink).expect("PCM_Sink_CreateEx returned null");
        OwnedPcmSink::from_raw(sink)
    }
}

impl AudioSupplier for Recorder {
    fn supply_audio(
        &mut self,
        request: &SupplyAudioRequest,
        dest_buffer: &mut AudioBufMut,
    ) -> SupplyResponse {
        let source = match self.state.as_mut().unwrap() {
            State::Ready(s) => &mut s.source,
            State::Recording(s) => {
                // TODO-high Implement last audio moments
                return SupplyResponse {
                    num_frames_written: 0,
                    num_frames_consumed: 0,
                    next_inner_frame: None,
                };
            }
        };
        return source.supply_audio(request, dest_buffer);
        // // TODO-high Obviously just some experiments.
        // let temp_buf = self.temporary_audio_buffer.to_buf();
        // if request.start_frame < 0 {
        //     return self.supplier.supply_audio(request, dest_buffer);
        // }
        // let mod_start_frame = request.start_frame as usize % temp_buf.frame_count();
        // let ideal_end_frame = mod_start_frame + dest_buffer.frame_count();
        // let end_frame = cmp::min(ideal_end_frame, temp_buf.frame_count());
        // let num_frames_to_write = end_frame - mod_start_frame;
        // temp_buf
        //     .slice(mod_start_frame..end_frame)
        //     .copy_to(&mut dest_buffer.slice_mut(..num_frames_to_write))
        //     .unwrap();
        // let num_frames_written = dest_buffer.frame_count();
        // SupplyResponse {
        //     num_frames_written,
        //     num_frames_consumed: num_frames_written,
        //     next_inner_frame: Some(request.start_frame + num_frames_written as isize),
        // }
    }

    fn channel_count(&self) -> usize {
        match self.state.as_ref().unwrap() {
            State::Ready(s) => s.source.channel_count(),
            // TODO-high Implement last audio moments
            State::Recording(_) => 0,
        }
    }
}

impl MidiSupplier for Recorder {
    fn supply_midi(
        &mut self,
        request: &SupplyMidiRequest,
        event_list: &BorrowedMidiEventList,
    ) -> SupplyResponse {
        let source = match self.state.as_mut().unwrap() {
            State::Ready(s) => &mut s.source,
            State::Recording(s) => match &mut s.sub_state {
                RecordingSubState::Audio(_) => {
                    return SupplyResponse {
                        num_frames_written: 0,
                        num_frames_consumed: 0,
                        next_inner_frame: None,
                    }
                }
                RecordingSubState::Midi(s) => &mut s.new_source,
            },
        };
        source.supply_midi(request, event_list)
    }
}

impl ExactFrameCount for Recorder {
    fn frame_count(&self) -> usize {
        match self.state.as_ref().unwrap() {
            State::Ready(s) => s.source.frame_count(),
            // TODO-high Implement last audio moments
            State::Recording(_) => 0,
        }
    }
}

impl ExactDuration for Recorder {
    fn duration(&self) -> DurationInSeconds {
        match self.state.as_ref().unwrap() {
            State::Ready(s) => s.source.duration(),
            // TODO-high Implement last audio moments
            State::Recording(_) => DurationInSeconds::MIN,
        }
    }
}

impl WithFrameRate for Recorder {
    fn frame_rate(&self) -> Option<Hz> {
        match self.state.as_ref().unwrap() {
            State::Ready(s) => s.source.frame_rate(),
            // TODO-high Implement last audio moments
            State::Recording(_) => None,
        }
    }
}
