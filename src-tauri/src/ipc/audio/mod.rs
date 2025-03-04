mod device_changer;
pub mod notifier;

// https://qiita.com/benki/items/635867b654783da0322f

use anyhow::Result;
use std::{collections::HashMap, ffi::OsString, os::windows::ffi::OsStringExt, sync::Arc};
use tokio::sync::mpsc::Sender;
use windows::{
    core::Interface,
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Foundation::{CloseHandle, FALSE},
        Media::Audio::{
            eMultimedia, eRender, Endpoints::IAudioEndpointVolume, IAudioSessionControl,
            IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator,
            ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
        },
        System::{
            Com::{
                CoCreateInstance, CoInitialize, CoUninitialize,
                StructuredStorage::PropVariantToStringAlloc, CLSCTX_ALL, STGM_READ,
            },
            ProcessStatus::GetModuleBaseNameW,
            Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
        },
    },
};

struct Com;

impl Com {
    pub fn new() -> Result<Self> {
        unsafe {
            let _ = CoInitialize(None);
        }

        Ok(Com)
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

pub struct Singleton {
    _com: Com,

    /// @see https://learn.microsoft.com/ja-jp/windows/win32/api/mmdeviceapi/nn-mmdeviceapi-immdeviceenumerator
    pub(crate) device_enumerator: IMMDeviceEnumerator,
    notification_callbacks: notifier::NotificationCallbacks,
    policy_config: device_changer::PolicyConfig,
}

unsafe impl Send for Singleton {}
unsafe impl Sync for Singleton {}

impl Singleton {
    pub fn new(tx: &Sender<notifier::Notification>) -> Result<Self> {
        let com = Com::new()?;
        let device_enumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let notification_callbacks = notifier::NotificationCallbacks::new(tx);
        notification_callbacks.register_to_enumerator(&device_enumerator)?;

        let policy_config = device_changer::PolicyConfig::new()?;

        Ok(Singleton {
            _com: com,
            device_enumerator,
            notification_callbacks,
            policy_config,
        })
    }

    pub fn get_active_audio_devices(self: &Arc<Self>) -> Result<Vec<IMMAudioDevice>> {
        // https://learn.microsoft.com/ja-jp/windows/win32/api/mmdeviceapi/nn-mmdeviceapi-immdevicecollection
        let device_collection = unsafe {
            self.device_enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?
        };

        let len = unsafe { device_collection.GetCount()? };

        let devices = (0..len)
            .map(|i| {
                let device = unsafe { device_collection.Item(i)? };
                IMMAudioDevice::new(Arc::clone(self), device)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(devices)
    }

    pub fn get_default_audio_id(&self) -> Result<String> {
        let device = unsafe {
            self.device_enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)?
        };
        let id = unsafe { device.GetId()?.to_string()? };

        Ok(id)
    }
}

impl Drop for Singleton {
    fn drop(&mut self) {
        self.notification_callbacks
            .unregister_to_enumerator(&self.device_enumerator)
            .unwrap();
    }
}

fn get_name_from_immdevice(device: &IMMDevice) -> Result<String> {
    let property_store = unsafe { device.OpenPropertyStore(STGM_READ)? };
    let name_propvariant = unsafe { property_store.GetValue(&PKEY_Device_FriendlyName)? };
    let name = unsafe { PropVariantToStringAlloc(&name_propvariant)?.to_string()? };

    Ok(name)
}

pub struct IMMAudioDevice {
    is: Arc<Singleton>,

    pub id: String,
    pub name: String,

    /// @see https://learn.microsoft.com/ja-jp/windows/win32/api/mmdeviceapi/nn-mmdeviceapi-immdevice
    _device: IMMDevice,

    /// @see https://learn.microsoft.com/ja-jp/windows/win32/api/endpointvolume/nn-endpointvolume-iaudioendpointvolume
    pub(crate) endpoint_volume: IAudioEndpointVolume,

    pub(crate) session_control_map: HashMap<u32, IAudioSessionControl>,
}

unsafe impl Send for IMMAudioDevice {}
unsafe impl Sync for IMMAudioDevice {}

impl IMMAudioDevice {
    pub fn new(is: Arc<Singleton>, device: IMMDevice) -> Result<Self> {
        let id = unsafe { device.GetId()?.to_string()? };
        let name = get_name_from_immdevice(&device)?;

        // https://learn.microsoft.com/ja-jp/windows/win32/api/mmdeviceapi/nf-mmdeviceapi-immdevice-activate
        // https://learn.microsoft.com/ja-jp/windows/win32/api/endpointvolume/nn-endpointvolume-iaudioendpointvolume
        let endpoint_volume: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None)? };

        let mut session_control_map: HashMap<u32, IAudioSessionControl> = HashMap::new();

        #[cfg(debug_assertions)]
        unsafe {
            let session_manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let sessions = session_manager.GetSessionEnumerator()?;

            for i in 0..sessions.GetCount()? {
                let session_control: IAudioSessionControl = sessions.GetSession(i)?;
                let session_control2: IAudioSessionControl2 = session_control.cast().unwrap();
                let process_id = session_control2.GetProcessId()?;

                session_control_map.insert(process_id, session_control);

                // let mut process_name = "Unknown".to_string();

                // if process_id == 0 {
                //     process_name = "System Sounds".to_string();
                // } else if let Ok(name) = get_process_name_by_id(process_id) {
                //     process_name = name;
                // }

                // let audio_volume: ISimpleAudioVolume = session_control.cast().unwrap();
                // let mut volume = 0.0;
                // let mut mute = false;

                // if let Ok(v) = audio_volume.GetMasterVolume() {
                //     volume = v;
                // }

                // if let Ok(m) = audio_volume.GetMute() {
                //     mute = m.as_bool();
                // }

                // println!(
                //     "Session: {} (PID: {}), Volume: {}, Mute: {}",
                //     process_name, process_id, volume, mute
                // );
            }
        }

        is.notification_callbacks
            .register_to_volume(&endpoint_volume)?;

        Ok(IMMAudioDevice {
            id,
            name,
            _device: device,
            endpoint_volume,
            is,
            session_control_map,
        })
    }

    pub(crate) fn get_session(&self, process_id: u32) -> Result<IAudioSessionControl> {
        let session_control = self.session_control_map.get(&process_id).unwrap();
        Ok(session_control.clone())
    }

    pub(crate) fn get_session_audio_volume(&self, process_id: u32) -> Result<ISimpleAudioVolume> {
        let session_control = self.get_session(process_id)?;
        let audio_volume: ISimpleAudioVolume = session_control.cast().unwrap();
        Ok(audio_volume)
    }

    pub fn set_as_default(&self) -> Result<()> {
        self.is.policy_config.set_default_endpoint(&self.id)?;

        Ok(())
    }

    pub fn get_volume(&self) -> Result<f32> {
        let volume = unsafe { self.endpoint_volume.GetMasterVolumeLevelScalar()? };

        Ok(volume)
    }

    pub fn get_session_volume(&self, process_id: u32) -> Result<f32> {
        let audio_volume = self.get_session_audio_volume(process_id)?;
        let volume = unsafe { audio_volume.GetMasterVolume()? };

        Ok(volume)
    }

    pub fn get_mute_state(&self) -> Result<bool> {
        let mute_state = unsafe { self.endpoint_volume.GetMute()?.as_bool() };

        Ok(mute_state)
    }

    pub fn get_session_mute_state(&self, process_id: u32) -> Result<bool> {
        let audio_volume = self.get_session_audio_volume(process_id)?;
        let mute_state = unsafe { audio_volume.GetMute()?.as_bool() };

        Ok(mute_state)
    }

    pub fn set_volume(&self, volume: f32) -> Result<()> {
        unsafe {
            self.endpoint_volume
                .SetMasterVolumeLevelScalar(volume, std::ptr::null())?;
        }

        Ok(())
    }

    pub fn set_session_volume(&self, process_id: u32, volume: f32) -> Result<()> {
        let audio_volume = self.get_session_audio_volume(process_id)?;
        unsafe {
            audio_volume.SetMasterVolume(volume, std::ptr::null())?;
        }

        Ok(())
    }

    pub fn set_mute_state(&self, mute_state: bool) -> Result<()> {
        unsafe {
            self.endpoint_volume.SetMute(mute_state, std::ptr::null())?;
        }

        Ok(())
    }

    pub fn set_session_mute_state(&self, process_id: u32, mute_state: bool) -> Result<()> {
        let audio_volume = self.get_session_audio_volume(process_id)?;
        unsafe {
            audio_volume.SetMute(mute_state, std::ptr::null())?;
        }

        Ok(())
    }
}

impl Drop for IMMAudioDevice {
    fn drop(&mut self) {
        self.is
            .notification_callbacks
            .unregister_to_volume(&self.endpoint_volume)
            .unwrap();
    }
}

unsafe fn get_process_name_by_id(process_id: u32) -> Result<String> {
    let try_process_handle = OpenProcess(
        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
        FALSE,
        process_id,
    );

    if let Err(e) = try_process_handle {
        return Err(anyhow::anyhow!("Failed to open process: {}", e));
    }

    let process_handle = try_process_handle.unwrap();

    let mut buffer = [0; 1024];
    let len = GetModuleBaseNameW(process_handle, None, &mut buffer);

    let os_string = OsString::from_wide(&buffer[..len as usize]);
    let process_name = os_string.to_string_lossy().into_owned();

    CloseHandle(process_handle).unwrap();

    Ok(process_name)
}
