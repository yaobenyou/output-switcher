use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use tokio::sync::mpsc::Sender;

use super::{
    audio::{notifier::Notification, IMMAudioDevice, Singleton},
    error::{APIError, UnexpectedErr},
};

pub type AudioDeviceMap = BTreeMap<String, IMMAudioDevice>;

#[derive(serde::Serialize, Debug, Clone)]
pub struct AudioDeviceInfo {
    id: String,
    name: String,
    volume: f32,
    muted: bool,
}

impl AudioDeviceInfo {
    fn from_audio(audio: &IMMAudioDevice) -> Result<Self> {
        Ok(Self {
            id: audio.id.clone(),
            name: audio.name.clone(),
            volume: audio.get_volume()?,
            muted: audio.get_mute_state()?,
        })
    }
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowsAudioState {
    audio_device_list: Vec<AudioDeviceInfo>,
    default: String,
}

impl WindowsAudioState {
    fn new(audio_dict: &AudioDeviceMap, default: String) -> Result<Self> {
        let audio_device_list = audio_dict
            .values()
            .map(|a| AudioDeviceInfo::from_audio(a))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            audio_device_list,
            default,
        })
    }
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioStateChangePayload {
    windows_audio_state: WindowsAudioState,
    notification: Option<Notification>,
}

pub async fn ipc_sender(
    is: &Arc<Singleton>,
    audio_dict: &Arc<Mutex<AudioDeviceMap>>,
    notification: Option<Notification>,
    tx: &Sender<AudioStateChangePayload>,
) -> Result<()> {
    let default = is.get_default_audio_id()?;
    let audio_state = {
        let dict = audio_dict.lock().map_err(|_| APIError::Unexpected {
            inner: UnexpectedErr::LockError,
        })?;
        WindowsAudioState::new(&dict, default)?
    };

    let payload = AudioStateChangePayload {
        windows_audio_state: audio_state,
        notification,
    };

    tx.send(payload).await.map_err(|_| APIError::Unexpected {
        inner: UnexpectedErr::MPSCClosedError,
    })?;

    Ok(())
}
