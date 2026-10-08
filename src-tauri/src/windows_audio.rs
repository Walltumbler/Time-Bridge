use crate::{audio::AudioDevice, model::Result};
use std::{cell::RefCell, mem::size_of};
use windows::{core::{Interface, PCWSTR}, Win32::{Media::Audio::*, System::Com::*}};

struct ComGuard;
impl ComGuard {
    fn new() -> Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().map_err(|e| e.to_string())?; }
        Ok(Self)
    }
}
impl Drop for ComGuard { fn drop(&mut self) { unsafe { CoUninitialize(); } } }

fn enumerator() -> Result<IMMDeviceEnumerator> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string()) }
}
fn wave_devices() -> Vec<(u32, AudioDevice)> {
    let mut devices = Vec::new();
    unsafe {
        for index in 0..waveOutGetNumDevs() {
            // Documented winmm endpoint-ID queries; the argument is a device index.
            let handle = Some(HWAVEOUT(index as usize as *mut _));
            let mut bytes = 0u32;
            if waveOutMessage(handle, 2066, &mut bytes as *mut _ as usize, 0) != 0 || bytes == 0 || bytes > 65536 { continue; }
            let mut id = vec![0u16; (bytes as usize + 1) / 2];
            if waveOutMessage(handle, 2065, id.as_mut_ptr() as usize, bytes as usize) != 0 { continue; }
            let mut caps = WAVEOUTCAPSW::default();
            if waveOutGetDevCapsW(index as usize, &mut caps, size_of::<WAVEOUTCAPSW>() as u32) != 0 { continue; }
            let name = caps.szPname;
            devices.push((index, AudioDevice {
                id: String::from_utf16_lossy(&id[..id.iter().position(|&c| c == 0).unwrap_or(id.len())]),
                name: String::from_utf16_lossy(&name[..name.iter().position(|&c| c == 0).unwrap_or(name.len())]),
            }));
        }
    }
    devices
}
pub(crate) fn devices() -> Vec<AudioDevice> {
    let mut devices: Vec<_> = wave_devices().into_iter().map(|(_, d)| d).collect();
    let Ok(_com) = ComGuard::new() else { return devices; };
    let Ok(enumerator) = enumerator() else { return devices; };
    for device in &mut devices {
        if let Ok(name) = friendly_name(&enumerator, &device.id) { device.name = name; }
    }
    // Identically named monitors still need distinct choices.
    let names: Vec<_> = devices.iter().map(|d| d.name.clone()).collect();
    for (index, device) in devices.iter_mut().enumerate() {
        if names.iter().filter(|n| *n == &device.name).count() > 1 {
            device.name = format!("{} ({})", device.name, index + 1);
        }
    }
    devices
}
fn friendly_name(enumerator: &IMMDeviceEnumerator, id: &str) -> Result<String> {
    use windows::Win32::{Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc}};
    let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let device = enumerator.GetDevice(PCWSTR(wide.as_ptr())).map_err(|e| e.to_string())?;
        let store = device.OpenPropertyStore(STGM_READ).map_err(|e| e.to_string())?;
        let mut value = store.GetValue(&PKEY_Device_FriendlyName).map_err(|e| e.to_string())?;
        let name = PropVariantToStringAlloc(&value);
        let _ = PropVariantClear(&mut value);
        let name = name.map_err(|e| e.to_string())?;
        let result = name.to_string().map_err(|e| e.to_string());
        CoTaskMemFree(Some(name.as_ptr() as *const _));
        result
    }
}

pub(crate) fn resolve_device(preferred: Option<&str>) -> Result<(String, bool)> {
    let _com = ComGuard::new()?;
    unsafe {
        let enumerator = enumerator()?;
        if let Some(id) = preferred {
            let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
            if let Ok(device) = enumerator.GetDevice(PCWSTR(wide.as_ptr())) {
                if device.GetState().is_ok_and(|s| s == DEVICE_STATE_ACTIVE) && wave_devices().iter().any(|(_,d)| d.id == id) {
                    return Ok((id.to_owned(), false));
                }
            }
        }
        let endpoint = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).map_err(|e| e.to_string())?;
        let id = endpoint.GetId().map_err(|e| e.to_string())?;
        let result = id.to_string().map_err(|e| e.to_string());
        CoTaskMemFree(Some(id.as_ptr() as *const _));
        Ok((result?, preferred.is_some()))
    }
}

fn mm_result(code: u32) -> Result<()> {
    if code == 0 { return Ok(()); }
    let mut message = [0u16; 256];
    unsafe { waveOutGetErrorTextW(code, &mut message); }
    Err(format!("Alarm playback failed: {} ({code})", String::from_utf16_lossy(&message[..message.iter().position(|&c| c == 0).unwrap_or(message.len())])))
}

fn pcm_wave(bytes: &[u8]) -> Result<(WAVEFORMATEX, Vec<u8>)> {
    if bytes.get(..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") { return Err("Invalid bundled alarm waveform".into()); }
    let mut format = None;
    let mut data = None;
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let length = u32::from_le_bytes(bytes[offset+4..offset+8].try_into().unwrap()) as usize;
        let chunk = bytes.get(offset+8..offset+8+length).ok_or("Truncated alarm waveform")?;
        match &bytes[offset..offset+4] {
            b"fmt " if chunk.len() >= 16 => {
                let u16_at = |i| u16::from_le_bytes(chunk[i..i+2].try_into().unwrap());
                let u32_at = |i| u32::from_le_bytes(chunk[i..i+4].try_into().unwrap());
                if u16_at(0) != 1 { return Err("Alarm waveform must be PCM".into()); }
                format = Some(WAVEFORMATEX { wFormatTag: 1, nChannels: u16_at(2), nSamplesPerSec: u32_at(4),
                    nAvgBytesPerSec: u32_at(8), nBlockAlign: u16_at(12), wBitsPerSample: u16_at(14), cbSize: 0 });
            }
            b"data" => data = Some(chunk.to_vec()),
            _ => {}
        }
        offset += 8 + length + length % 2;
    }
    Ok((format.ok_or("Missing alarm format")?, data.filter(|d| !d.is_empty()).ok_or("Missing alarm samples")?))
}

struct Player { handle: HWAVEOUT, header: Box<WAVEHDR>, _data: Vec<u8>, prepared: bool }
impl Player {
    fn start(sound: &[u8], device: Option<&str>, repeat: bool) -> Result<Self> {
        let index = match device {
            Some(id) => wave_devices().into_iter().find(|(_,d)| d.id == id).map(|(i,_)| i).ok_or("The alarm output device is unavailable")?,
            None => WAVE_MAPPER,
        };
        let (format, mut data) = pcm_wave(sound)?;
        let mut handle = HWAVEOUT::default();
        unsafe { mm_result(waveOutOpen(Some(&mut handle), index, &format, None, None, CALLBACK_NULL))?; }
        let header = Box::new(WAVEHDR { lpData: windows::core::PSTR(data.as_mut_ptr()), dwBufferLength: data.len() as u32,
            dwFlags: if repeat { WHDR_BEGINLOOP | WHDR_ENDLOOP } else { 0 }, dwLoops: if repeat { u32::MAX } else { 0 }, ..Default::default() });
        let mut player = Self { handle, header, _data: data, prepared: false };
        unsafe {
            mm_result(waveOutPrepareHeader(handle, &mut *player.header, size_of::<WAVEHDR>() as u32))?;
            player.prepared = true;
            mm_result(waveOutWrite(handle, &mut *player.header, size_of::<WAVEHDR>() as u32))?;
        }
        Ok(player)
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        unsafe {
            // Reset returns queued buffers before we free the pinned header/data.
            waveOutReset(self.handle);
            if self.prepared { waveOutUnprepareHeader(self.handle, &mut *self.header, size_of::<WAVEHDR>() as u32); }
            waveOutClose(self.handle);
        }
    }
}
thread_local! { static PLAYER: RefCell<Option<Player>> = const { RefCell::new(None) }; }
pub(crate) fn play(sound: &[u8], device: Option<&str>, repeat: bool) -> Result<()> {
    stop();
    let player = Player::start(sound, device, repeat)?;
    PLAYER.with(|p| *p.borrow_mut() = Some(player));
    Ok(())
}
pub(crate) fn stop() { PLAYER.with(|p| *p.borrow_mut() = None); }

struct SavedSession { id: String, control: ISimpleAudioVolume, muted: bool }
pub(crate) struct OtherApps { saved: Vec<SavedSession>, _com: ComGuard }
impl OtherApps {
    pub(crate) fn new() -> Result<Self> { Ok(Self { saved: Vec::new(), _com: ComGuard::new()? }) }
    pub(crate) fn mute(&mut self) -> Result<()> {
        unsafe {
            let endpoints = enumerator()?.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE).map_err(|e| e.to_string())?;
            for index in 0..endpoints.GetCount().map_err(|e| e.to_string())? {
                let device = endpoints.Item(index).map_err(|e| e.to_string())?;
                let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
                let sessions = manager.GetSessionEnumerator().map_err(|e| e.to_string())?;
                for index in 0..sessions.GetCount().map_err(|e| e.to_string())? {
                    let session = sessions.GetSession(index).map_err(|e| e.to_string())?;
                    let details: IAudioSessionControl2 = session.cast().map_err(|e| e.to_string())?;
                    // Do not change a session unless we can establish that it belongs to another process.
                    let Ok(pid) = details.GetProcessId() else { continue; };
                    if pid == std::process::id() { continue; }
                    let raw_id = details.GetSessionInstanceIdentifier().map_err(|e| e.to_string())?;
                    let id = raw_id.to_string().map_err(|e| e.to_string());
                    CoTaskMemFree(Some(raw_id.as_ptr() as *const _));
                    let id = id?;
                    if self.saved.iter().any(|s| s.id == id) { continue; }
                    let control: ISimpleAudioVolume = session.cast().map_err(|e| e.to_string())?;
                    let muted = control.GetMute().map_err(|e| e.to_string())?.as_bool();
                    // Save before mutation so a failed mute can still be rolled back.
                    self.saved.push(SavedSession { id, control: control.clone(), muted });
                    control.SetMute(true, std::ptr::null()).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn restore(&mut self) -> Result<()> {
        let mut errors = Vec::new();
        self.saved.retain(|session| {
            match unsafe { session.control.SetMute(session.muted, std::ptr::null()) } {
                Ok(()) => false,
                Err(error) => { errors.push(error.to_string()); true }
            }
        });
        if errors.is_empty() { Ok(()) } else { Err(format!("Could not restore other apps: {}", errors.join("; "))) }
    }
}
impl Drop for OtherApps { fn drop(&mut self) { let _ = self.restore(); } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Temporarily mutes real desktop audio sessions; run explicitly on an interactive desktop"]
    fn desktop_audio_other_apps_restore() {
        let mut apps = OtherApps::new().unwrap();
        apps.mute().unwrap();
        let saved: Vec<_> = apps.saved.iter().map(|s| (s.control.clone(), s.muted)).collect();
        println!("Checked {} other audio sessions", saved.len());
        for (control, _) in &saved { assert!(unsafe { control.GetMute().unwrap().as_bool() }); }
        apps.mute().unwrap();
        apps.restore().unwrap();
        for (control, muted) in saved { assert_eq!(unsafe { control.GetMute().unwrap().as_bool() }, muted); }
    }
    #[test]
    fn bundled_waveform_is_valid_and_truncation_is_rejected() {
        let sound = include_bytes!("../assets/alert.wav");
        let (format, data) = pcm_wave(sound).unwrap();
        let channels = format.nChannels;
        let block = format.nBlockAlign;
        assert!(channels > 0 && block > 0);
        assert_eq!(data.len() % block as usize, 0);
        assert!(pcm_wave(&sound[..sound.len()-1]).is_err());
    }
}
