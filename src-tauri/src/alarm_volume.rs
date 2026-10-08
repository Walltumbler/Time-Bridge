//! A single lease spans all overlapping alarms. The snapshot belongs to the original
//! endpoint, so changing the default device cannot restore settings onto another device.
use crate::model::Result;

pub(crate) trait Snapshot: Sized {
    fn capture(device: Option<&str>) -> Result<Self>;
    fn apply(&self, percent: u8) -> Result<()>;
    fn restore(&self) -> Result<()>;
}

pub(crate) struct Lease<S = PlatformSnapshot> {
    saved: Option<S>,
    percent: Option<u8>,
    device: Option<String>,
}
impl<S> Default for Lease<S> {
    fn default() -> Self { Self { saved: None, percent: None, device: None } }
}

impl<S: Snapshot> Lease<S> {
    pub(crate) fn update(&mut self, percent: Option<u8>, device: Option<&str>) -> Result<()> {
        if percent.is_some_and(|p| !(1..=100).contains(&p)) {
            return Err("Alarm volume must be between 1 and 100 percent".into());
        }
        if self.saved.is_some() && self.device.as_deref() != device { self.release()?; }
        if percent == self.percent && (percent.is_some() || self.saved.is_none()) { return Ok(()); }
        if let Some(percent) = percent {
            if self.saved.is_none() { self.saved = Some(S::capture(device)?); self.device = device.map(str::to_owned); }
            if let Err(error) = self.saved.as_ref().unwrap().apply(percent) {
                self.percent = None;
                return match self.release() {
                    Ok(()) => Err(error),
                    Err(restore) => Err(format!("{error}; restoring volume also failed: {restore}")),
                };
            }
            self.percent = Some(percent);
            Ok(())
        } else { self.release() }
    }

    fn release(&mut self) -> Result<()> {
        if let Some(saved) = &self.saved { saved.restore()?; }
        self.saved = None;
        self.device = None;
        self.percent = None;
        Ok(())
    }
}

#[cfg(windows)]
pub(crate) use platform::WindowsSnapshot as PlatformSnapshot;
#[cfg(windows)]
mod platform {
    use super::*;
    use windows::Win32::{Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eRender, eMultimedia,
        Endpoints::IAudioEndpointVolume, IAudioSessionManager, ISimpleAudioVolume},
        System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED}};

    struct ComGuard;
    impl Drop for ComGuard { fn drop(&mut self) { unsafe { CoUninitialize(); } } }

    pub(crate) struct WindowsSnapshot {
        endpoint: IAudioEndpointVolume,
        session: ISimpleAudioVolume,
        volume: f32,
        muted: bool,
        app_volume: f32,
        app_muted: bool,
        // COM interfaces above must be released before the apartment is uninitialized.
        _com: ComGuard,
    }

    impl Snapshot for WindowsSnapshot {
        fn capture(device: Option<&str>) -> Result<Self> {
            unsafe {
                CoInitializeEx(None, COINIT_MULTITHREADED).ok().map_err(|e| e.to_string())?;
                let com = ComGuard;
                let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
                let device = if let Some(id) = device {
                    let id: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
                    enumerator.GetDevice(windows::core::PCWSTR(id.as_ptr()))
                } else { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }.map_err(|e| e.to_string())?;
                let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
                let manager: IAudioSessionManager = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
                let session = manager.GetSimpleAudioVolume(None, 0).map_err(|e| e.to_string())?;
                Ok(Self {
                    volume: endpoint.GetMasterVolumeLevelScalar().map_err(|e| e.to_string())?,
                    muted: endpoint.GetMute().map_err(|e| e.to_string())?.as_bool(),
                    app_volume: session.GetMasterVolume().map_err(|e| e.to_string())?,
                    app_muted: session.GetMute().map_err(|e| e.to_string())?.as_bool(),
                    endpoint, session, _com: com,
                })
            }
        }

        fn apply(&self, percent: u8) -> Result<()> {
            unsafe {
                self.endpoint.SetMasterVolumeLevelScalar(percent as f32 / 100.0, std::ptr::null()).map_err(|e| e.to_string())?;
                self.session.SetMasterVolume(1.0, std::ptr::null()).map_err(|e| e.to_string())?;
                self.session.SetMute(false, std::ptr::null()).map_err(|e| e.to_string())?;
                self.endpoint.SetMute(false, std::ptr::null()).map_err(|e| e.to_string())?;
            }
            Ok(())
        }

        fn restore(&self) -> Result<()> {
            // Attempt every field even if the endpoint has become unavailable.
            let results = unsafe { [
                self.endpoint.SetMute(self.muted, std::ptr::null()),
                self.endpoint.SetMasterVolumeLevelScalar(self.volume, std::ptr::null()),
                self.session.SetMute(self.app_muted, std::ptr::null()),
                self.session.SetMasterVolume(self.app_volume, std::ptr::null()),
            ] };
            let errors: Vec<_> = results.into_iter().filter_map(|r| r.err().map(|e| e.to_string())).collect();
            if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        #[ignore = "Changes desktop volume and plays a short alarm; run explicitly on an interactive desktop"]
        fn desktop_audio_volume_and_routing_restore() {
            struct Restore(WindowsSnapshot);
            impl Drop for Restore { fn drop(&mut self) { let _ = crate::audio::sync_alarm(false, None, None, false); let _ = self.0.restore(); } }
            let devices = crate::audio::devices();
            assert!(!devices.is_empty(), "No playback devices found");
            println!("Playback devices: {}", serde_json::to_string(&devices).unwrap());
            let (default_before, _) = crate::windows_audio::resolve_device(None).unwrap();
            let selected = devices.iter().find(|d| d.id != default_before).unwrap_or(&devices[0]).id.clone();
            let restore = Restore(WindowsSnapshot::capture(Some(&selected)).unwrap());
            crate::audio::sync_alarm(true, Some(75), Some(selected.clone()), false).unwrap();
            unsafe {
                assert!((restore.0.endpoint.GetMasterVolumeLevelScalar().unwrap() - 0.75).abs() < 0.01);
                assert!(!restore.0.endpoint.GetMute().unwrap().as_bool());
                assert!(!restore.0.session.GetMute().unwrap().as_bool());
                assert!((restore.0.session.GetMasterVolume().unwrap() - 1.0).abs() < 0.01);
            }
            std::thread::sleep(std::time::Duration::from_millis(400));
            // Repeated ticks and an overlapping alarm must not replace the original snapshot.
            crate::audio::sync_alarm(true, Some(75), Some(selected), false).unwrap();
            crate::audio::sync_alarm(false, None, None, false).unwrap();
            unsafe {
                assert!((restore.0.endpoint.GetMasterVolumeLevelScalar().unwrap() - restore.0.volume).abs() < 0.001);
                assert_eq!(restore.0.endpoint.GetMute().unwrap().as_bool(), restore.0.muted);
                assert!((restore.0.session.GetMasterVolume().unwrap() - restore.0.app_volume).abs() < 0.001);
                assert_eq!(restore.0.session.GetMute().unwrap().as_bool(), restore.0.app_muted);
            }
            assert_eq!(crate::windows_audio::resolve_device(None).unwrap().0, default_before);
        }
    }
}

#[cfg(not(windows))]
#[derive(Default)]
pub(crate) struct PlatformSnapshot;
#[cfg(not(windows))]
impl Snapshot for PlatformSnapshot {
    fn capture(_device: Option<&str>) -> Result<Self> { Err("Automatic alarm volume is currently supported on Windows".into()) }
    fn apply(&self, _: u8) -> Result<()> { Ok(()) }
    fn restore(&self) -> Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[derive(Clone, Debug, PartialEq)]
    struct Fake { volume: u8, muted: bool, app_volume: u8, app_muted: bool }
    thread_local! { static STATE: RefCell<Fake> = const { RefCell::new(Fake { volume: 23, muted: true, app_volume: 42, app_muted: true }) }; }
    impl Snapshot for Fake {
        fn capture(_device: Option<&str>) -> Result<Self> { Ok(STATE.with(|s| s.borrow().clone())) }
        fn apply(&self, percent: u8) -> Result<()> {
            STATE.with(|s| *s.borrow_mut() = Fake { volume: percent, muted: false, app_volume: 100, app_muted: false });
            if percent == 99 { Err("Simulated partial failure".into()) } else { Ok(()) }
        }
        fn restore(&self) -> Result<()> { STATE.with(|s| *s.borrow_mut() = self.clone()); Ok(()) }
    }
    #[test]
    fn overlapping_alarms_and_preference_changes_preserve_original_snapshot() {
        let original = Fake::capture(None).unwrap();
        let mut lease = Lease::<Fake> { saved: None, percent: None, device: None };
        lease.update(Some(75), None).unwrap();
        lease.update(Some(75), None).unwrap();
        lease.update(Some(80), None).unwrap();
        assert_eq!(Fake::capture(None).unwrap().volume, 80);
        assert!(!Fake::capture(None).unwrap().muted);
        lease.update(None, None).unwrap();
        assert_eq!(Fake::capture(None).unwrap(), original);
        lease.update(None, None).unwrap();
        assert_eq!(Fake::capture(None).unwrap(), original);
    }
    #[test]
    fn partial_override_failure_rolls_back_every_field() {
        let original = Fake::capture(None).unwrap();
        let mut lease = Lease::<Fake> { saved: None, percent: None, device: None };
        assert!(lease.update(Some(99), None).is_err());
        assert_eq!(Fake::capture(None).unwrap(), original);
        assert!(lease.saved.is_none());
    }
}
