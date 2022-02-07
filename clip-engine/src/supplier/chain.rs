use crate::supplier::{Fader, Looper};
use crate::{
    ClipContent, ClipInfo, ExactDuration, ExactFrameCount, Recorder, Resampler, TimeStretcher,
    WithFrameRate,
};
use reaper_high::Project;
use reaper_medium::{DurationInSeconds, Hz, OwnedPcmSource};

type Head = FaderTail;
type FaderTail = Fader<ResamplerTail>;
type ResamplerTail = Resampler<TimeStretcherTail>;
type TimeStretcherTail = TimeStretcher<LooperTail>;
type LooperTail = Looper<RecorderTail>;
type RecorderTail = Recorder;
type SourceTail = OwnedPcmSource;

#[derive(Debug)]
pub struct SupplierChain {
    head: Head,
}

impl SupplierChain {
    pub fn new(recorder: Recorder) -> Self {
        let mut chain = Self {
            head: { Fader::new(Resampler::new(TimeStretcher::new(Looper::new(recorder)))) },
        };
        // Configure resampler
        let resampler = chain.resampler_mut();
        resampler.set_enabled(true);
        // Configure time stratcher
        let time_stretcher = chain.time_stretcher_mut();
        time_stretcher.set_enabled(true);
        // Configure looper
        let looper = chain.looper_mut();
        looper.set_fades_enabled(true);
        chain
    }

    pub fn head(&self) -> &Head {
        &self.head
    }

    pub fn head_mut(&mut self) -> &mut Head {
        &mut self.head
    }

    pub fn fader(&self) -> &FaderTail {
        &self.head
    }

    pub fn fader_mut(&mut self) -> &mut FaderTail {
        &mut self.head
    }

    pub fn resampler(&self) -> &ResamplerTail {
        self.head.supplier()
    }

    pub fn resampler_mut(&mut self) -> &mut ResamplerTail {
        self.head.supplier_mut()
    }

    pub fn time_stretcher(&self) -> &TimeStretcherTail {
        self.resampler().supplier()
    }

    pub fn time_stretcher_mut(&mut self) -> &mut TimeStretcherTail {
        self.resampler_mut().supplier_mut()
    }

    pub fn looper(&self) -> &LooperTail {
        self.time_stretcher().supplier()
    }

    pub fn looper_mut(&mut self) -> &mut LooperTail {
        self.time_stretcher_mut().supplier_mut()
    }

    pub fn recorder(&self) -> &RecorderTail {
        self.looper().supplier()
    }

    pub fn recorder_mut(&mut self) -> &mut RecorderTail {
        self.looper_mut().supplier_mut()
    }

    pub fn source_frame_rate_in_ready_state(&self) -> Hz {
        self.recorder()
            .frame_rate()
            .expect("recorder couldn't provide frame rate even though clip is in ready state")
    }

    pub fn source_frame_count_in_ready_state(&self) -> usize {
        self.recorder().frame_count()
    }

    pub fn source_duration_in_ready_state(&self) -> DurationInSeconds {
        self.recorder().duration()
    }

    pub fn clip_info(&self) -> Option<ClipInfo> {
        self.recorder().clip_info()
    }

    pub fn clip_content(&self, project: Option<Project>) -> Option<ClipContent> {
        self.recorder().clip_content(project)
    }
}
