use crate::multimedia::{
    audio::{AudioFormat, AudioOutput},
    AudioSpec, MultimediaError, Result,
};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_audio as gst_audio;
use std::sync::{Arc, Mutex};

pub struct Gtk4Audio {
    pipeline: Option<gst::Pipeline>,
    appsrc: Option<gst_app::AppSrc>,
    spec: Option<AudioSpec>,
    format: Option<AudioFormat>,
    volume: Arc<Mutex<f32>>,
    paused: Arc<Mutex<bool>>,
}

// Safety: GStreamer is thread-safe
unsafe impl Send for Gtk4Audio {}

impl Gtk4Audio {
    pub fn new() -> Result<Self> {
        // Initialize GStreamer
        gst::init()
            .map_err(|e| MultimediaError::new(format!("Failed to initialize GStreamer: {}", e)))?;

        Ok(Self {
            pipeline: None,
            appsrc: None,
            spec: None,
            format: None,
            volume: Arc::new(Mutex::new(1.0)),
            paused: Arc::new(Mutex::new(false)),
        })
    }

    fn gst_format_from_audio_format(format: AudioFormat) -> gst_audio::AudioFormat {
        match format {
            AudioFormat::U8 => gst_audio::AudioFormat::U8,
            AudioFormat::S16 => gst_audio::AudioFormat::S16le,
            AudioFormat::S32 => gst_audio::AudioFormat::S32le,
            AudioFormat::F32 => gst_audio::AudioFormat::F32le,
        }
    }

    fn create_pipeline(&mut self, spec: &AudioSpec, format: AudioFormat) -> Result<()> {
        // Create pipeline elements
        let pipeline = gst::Pipeline::new();
        let appsrc = gst_app::AppSrc::builder()
            .name("audio-source")
            .format(gst::Format::Time)
            .build();

        let audioconvert = gst::ElementFactory::make("audioconvert")
            .build()
            .map_err(|e| MultimediaError::new(format!("Failed to create audioconvert: {}", e)))?;

        let audioresample = gst::ElementFactory::make("audioresample")
            .build()
            .map_err(|e| MultimediaError::new(format!("Failed to create audioresample: {}", e)))?;

        let volume = gst::ElementFactory::make("volume")
            .build()
            .map_err(|e| MultimediaError::new(format!("Failed to create volume element: {}", e)))?;

        let audiosink = gst::ElementFactory::make("autoaudiosink")
            .build()
            .map_err(|e| MultimediaError::new(format!("Failed to create autoaudiosink: {}", e)))?;

        // Configure appsrc caps
        let gst_format = Self::gst_format_from_audio_format(format);
        let info = gst_audio::AudioInfo::builder(gst_format, spec.frequency, spec.channels as u32)
            .build()
            .map_err(|e| MultimediaError::new(format!("Failed to create audio info: {}", e)))?;

        let audio_caps = info
            .to_caps()
            .map_err(|e| MultimediaError::new(format!("Failed to create audio caps: {}", e)))?;

        appsrc.set_caps(Some(&audio_caps));
        appsrc.set_format(gst::Format::Time);

        // Add elements to pipeline
        pipeline
            .add_many(&[
                appsrc.upcast_ref(),
                &audioconvert,
                &audioresample,
                &volume,
                &audiosink,
            ])
            .map_err(|e| {
                MultimediaError::new(format!("Failed to add elements to pipeline: {}", e))
            })?;

        // Link elements
        gst::Element::link_many(&[
            appsrc.upcast_ref(),
            &audioconvert,
            &audioresample,
            &volume,
            &audiosink,
        ])
        .map_err(|e| MultimediaError::new(format!("Failed to link elements: {}", e)))?;

        // Set initial volume
        if let Ok(vol) = self.volume.lock() {
            volume.set_property("volume", *vol as f64);
        }

        self.pipeline = Some(pipeline);
        self.appsrc = Some(appsrc);

        Ok(())
    }
}

impl AudioOutput for Gtk4Audio {
    fn initialize(&mut self, spec: AudioSpec, format: AudioFormat) -> Result<()> {
        self.spec = Some(spec);
        self.format = Some(format);

        self.create_pipeline(&spec, format)?;

        if let Some(ref pipeline) = self.pipeline {
            pipeline
                .set_state(gst::State::Playing)
                .map_err(|e| MultimediaError::new(format!("Failed to start pipeline: {:?}", e)))?;
        }

        Ok(())
    }

    fn queue_samples(&mut self, samples: &[u8]) -> Result<()> {
        if self.is_paused() {
            return Ok(());
        }

        if let Some(ref appsrc) = self.appsrc {
            let buffer = gst::Buffer::from_slice(samples.to_vec());

            appsrc
                .push_buffer(buffer)
                .map_err(|e| MultimediaError::new(format!("Failed to push buffer: {:?}", e)))?;
        }

        Ok(())
    }

    fn pause(&mut self, paused: bool) -> Result<()> {
        if let Ok(mut p) = self.paused.lock() {
            *p = paused;
        }

        if let Some(ref pipeline) = self.pipeline {
            let state = if paused {
                gst::State::Paused
            } else {
                gst::State::Playing
            };
            pipeline.set_state(state).map_err(|e| {
                MultimediaError::new(format!("Failed to set pipeline state: {:?}", e))
            })?;
        }

        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<()> {
        let clamped_volume = volume.clamp(0.0, 1.0);

        if let Ok(mut vol) = self.volume.lock() {
            *vol = clamped_volume;
        }

        // Update volume element if pipeline is created
        if let Some(ref pipeline) = self.pipeline {
            if let Some(volume_element) = pipeline.by_name("volume0") {
                volume_element.set_property("volume", clamped_volume as f64);
            }
        }

        Ok(())
    }

    fn get_volume(&self) -> f32 {
        self.volume.lock().map(|v| *v).unwrap_or(1.0)
    }

    fn is_paused(&self) -> bool {
        self.paused.lock().map(|p| *p).unwrap_or(false)
    }

    fn get_queued_size(&self) -> usize {
        // GStreamer handles buffering internally
        0
    }

    fn clear_queue(&mut self) -> Result<()> {
        // Not directly supported in GStreamer pipeline
        // Would need to flush the pipeline if needed
        Ok(())
    }

    fn get_spec(&self) -> Option<&AudioSpec> {
        self.spec.as_ref()
    }
}

impl Drop for Gtk4Audio {
    fn drop(&mut self) {
        if let Some(ref pipeline) = self.pipeline {
            let _ = pipeline.set_state(gst::State::Null);
        }
    }
}
