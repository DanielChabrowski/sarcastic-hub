use anyhow::Result;
use gstreamer::prelude::*;
use log::{debug, error};

pub struct Player {
    inner: gstreamer_player::Player,
    _sink: gstreamer::Element,
}

impl Player {
    pub fn new() -> Result<Self> {
        gstreamer::init()?;

        let sink = gstreamer::ElementFactory::make("pulsesink", None)?;
        let inner = gstreamer_player::Player::new(None, None);

        inner.connect_error(|_, err| {
            error!("{:?}", err);
        });

        inner.connect_state_changed(|_, state| {
            debug!("state changed: {:?}", state);
        });

        inner.connect_buffering(|_, p| {
            debug!("buffering {}", p);
        });

        inner.connect_notify(Some("current-device"), |zelf, _| {
            let current_device = zelf.property("current-device").unwrap();
            use glib::GString;
            debug!(
                "current device changed: {:?}",
                current_device.get::<GString>().unwrap().as_str()
            );
        });

        let pipeline = inner.pipeline();
        pipeline.set_property("audio-sink", &sink)?;

        Ok(Self { inner, _sink: sink })
    }

    pub fn set_uri(&self, uri: &str) {
        self.inner.set_uri(uri);
    }

    pub fn play(&self) {
        self.inner.play();
    }

    pub fn stop(&self) {
        self.inner.stop();
    }

    pub fn pause(&self) {
        self.inner.pause();
    }

    // pub fn set_volume(&self, volume: f64) {
    //     self.inner.set_volume(volume);
    // }

    // pub fn set_audio_device(&self, device_name: impl ToString) {
    //     self.sink
    //         .set_property("device", &device_name.to_string())
    //         .unwrap();
    // }
}
