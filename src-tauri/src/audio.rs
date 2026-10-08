use serde::Serialize;
use crate::model::Result;

static SOUND: &[u8] = include_bytes!("../assets/alert.wav");
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct AudioStatus { pub available: bool, pub muted: bool, pub volume: Option<f32>, pub warning: Option<String> }

pub fn status(device: Option<String>) -> AudioStatus {
    match std::thread::spawn(move || read_status(device.as_deref())).join() {
        Ok(Ok(s)) => s,
        _ => AudioStatus { available:false, muted:false, volume:None, warning:Some("Output volume could not be checked. Use Test sound to check your speakers.".into()) },
    }
}
#[cfg(windows)]
fn read_status(_preferred: Option<&str>) -> Result<AudioStatus> {
    use windows::Win32::{Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, Endpoints::IAudioEndpointVolume, IAudioSessionManager, ISimpleAudioVolume}, System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED}};
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().map_err(|e| e.to_string())?;
        struct ComGuard; impl Drop for ComGuard { fn drop(&mut self) { unsafe { CoUninitialize(); } } }
        let _guard=ComGuard;
        let enumerator: IMMDeviceEnumerator=CoCreateInstance(&MMDeviceEnumerator,None,CLSCTX_ALL).map_err(|e|e.to_string())?;
        let (id,fallback)=crate::windows_audio::resolve_device(_preferred)?;
        let wide: Vec<u16>=id.encode_utf16().chain(Some(0)).collect();
        let device=enumerator.GetDevice(windows::core::PCWSTR(wide.as_ptr())).map_err(|e|e.to_string())?;
        let endpoint: IAudioEndpointVolume=device.Activate(CLSCTX_ALL,None).map_err(|e|e.to_string())?;
        let volume=endpoint.GetMasterVolumeLevelScalar().map_err(|e|e.to_string())?;
        let muted=endpoint.GetMute().map_err(|e|e.to_string())?.as_bool();
        let session: Option<ISimpleAudioVolume> = device.Activate::<IAudioSessionManager>(CLSCTX_ALL,None).ok().and_then(|m|m.GetSimpleAudioVolume(None,0).ok());
        let app_muted=session.as_ref().and_then(|s|s.GetMute().ok()).is_some_and(|m|m.as_bool());
        let app_zero=session.as_ref().and_then(|s|s.GetMasterVolume().ok()).is_some_and(|v|v<=0.001);
        let warning=if fallback {Some("Your chosen alarm output is disconnected. Alarms will use the current Windows output.".into())} else if muted || volume<=0.001 { Some("Your output is muted or its volume is zero. Timers will still show visual alerts.".into()) }
            else if app_muted || app_zero { Some("Timebridge is muted in the volume mixer. Timers will still show visual alerts.".into()) } else {None};
        Ok(AudioStatus { available:true, muted:muted||app_muted||app_zero, volume:Some(volume), warning })
    }
}
#[cfg(not(windows))]
fn read_status(_preferred: Option<&str>) -> Result<AudioStatus> {
    #[cfg(target_os="macos")]
    {
        let output=std::process::Command::new("/usr/bin/osascript").args(["-e","get volume settings"]).output().map_err(|e|e.to_string())?;
        if !output.status.success(){return Err("Could not inspect the macOS output device".into());}
        let settings=String::from_utf8_lossy(&output.stdout).to_lowercase();
        let volume=settings.split("output volume:").nth(1).and_then(|s|s.split(|c:char|!c.is_ascii_digit()).next()).and_then(|s|s.parse::<u8>().ok());
        let muted=settings.contains("output muted:true");
        let warning=if muted||volume==Some(0){Some("Your output is muted or its volume is zero. Timers will still show visual alerts.".into())}
            else if volume.is_none(){Some("Output volume could not be checked. Use Test sound to check your speakers.".into())}else{None};
        return Ok(AudioStatus{available:volume.is_some(),muted,volume:volume.map(|v|v as f32/100.0),warning});
    }
    #[cfg(not(target_os="macos"))]
    {
        #[cfg(target_os="linux")]
        {
            let mute=std::process::Command::new("pactl").args(["get-sink-mute","@DEFAULT_SINK@"]).output().map_err(|e|e.to_string())?;
            let level=std::process::Command::new("pactl").args(["get-sink-volume","@DEFAULT_SINK@"]).output().map_err(|e|e.to_string())?;
            if !mute.status.success()||!level.status.success(){return Err("Could not inspect the default PulseAudio/PipeWire output".into());}
            let muted=String::from_utf8_lossy(&mute.stdout).to_lowercase().contains("yes");
            let volume=String::from_utf8_lossy(&level.stdout).split_whitespace().find_map(|s|s.strip_suffix('%')?.parse::<u8>().ok()).map(|v|v as f32/100.0);
            let warning=if muted||volume==Some(0.0){Some("Your output is muted or its volume is zero. Timers will still show visual alerts.".into())}
                else if volume.is_none(){Some("Output volume could not be checked. Use Test sound to check your speakers.".into())}else{None};
            return Ok(AudioStatus{available:volume.is_some(),muted,volume,warning});
        }
        #[cfg(not(target_os="linux"))]
        { Err("Volume inspection is not supported on this platform yet".into()) }
    }
}

#[derive(Clone, Serialize)]
pub struct AudioDevice { pub id: String, pub name: String }
pub fn devices() -> Vec<AudioDevice> {
    #[cfg(windows)] { crate::windows_audio::devices() }
    #[cfg(not(windows))] { Vec::new() }
}

enum AudioCommand { Alarm(bool, Option<u8>, Option<String>, bool), Test(Option<String>), Shutdown }
type AudioRequest = (AudioCommand, std::sync::mpsc::Sender<Result<()>>);

#[derive(Default)]
struct AlarmAudio {
    lease: crate::alarm_volume::Lease,
    ringing: bool,
    device: Option<String>,
    last_sound: Option<std::time::Instant>,
    #[cfg(windows)]
    other_apps: Option<crate::windows_audio::OtherApps>,
}
impl AlarmAudio {
    fn stop(&mut self) -> Result<()> {
        if self.ringing { stop_raw(); }
        self.ringing = false;
        self.device = None;
        let volume = self.lease.update(None, None);
        #[cfg(windows)]
        let others = self.restore_other_apps();
        #[cfg(not(windows))]
        let others: Result<()> = Ok(());
        combine_errors([volume, others])
    }
    #[cfg(windows)]
    fn restore_other_apps(&mut self) -> Result<()> {
        if let Some(other_apps) = &mut self.other_apps { other_apps.restore()?; }
        self.other_apps = None;
        Ok(())
    }
    fn sync(&mut self, active: bool, percent: Option<u8>, preferred: Option<&str>, mute_others: bool) -> Result<()> {
        if !active { return self.stop(); }
        #[cfg(windows)]
        let (device, fallback) = match crate::windows_audio::resolve_device(preferred) {
            Ok((device, fallback)) => (Some(device), fallback),
            Err(error) => { let restored = self.stop(); return combine_errors([Err(error), restored]); }
        };
        #[cfg(not(windows))]
        let (device, fallback) = (preferred.map(str::to_owned), false);
        if self.ringing && self.device != device { self.stop()?; }
        let volume = self.lease.update(percent, device.as_deref());
        #[cfg(windows)]
        let others = if mute_others {
            match self.other_apps.as_mut() {
                Some(apps) => apps.mute(),
                None => match crate::windows_audio::OtherApps::new() {
                    Ok(apps) => { self.other_apps = Some(apps); self.other_apps.as_mut().unwrap().mute() }
                    Err(error) => Err(error),
                }
            }
        } else { self.restore_other_apps() };
        #[cfg(not(windows))]
        let others: Result<()> = if mute_others { Err("Muting other apps is currently supported on Windows".into()) } else { Ok(()) };
        let replay = !cfg!(windows) && self.last_sound.is_some_and(|t| t.elapsed().as_secs() >= 5);
        if !self.ringing || replay {
            if let Err(error) = play_raw(true, device.as_deref()) {
                let restored = self.stop();
                return combine_errors([Err(error), volume, others, restored]);
            }
            self.last_sound = Some(std::time::Instant::now());
        }
        self.ringing = true;
        self.device = device;
        let warning = if fallback { Err("Your chosen alarm output is disconnected. The alarm is using the current Windows output instead.".into()) } else { Ok(()) };
        combine_errors([volume, others, warning])
    }
}
impl Drop for AlarmAudio { fn drop(&mut self) { let _ = self.stop(); } }
fn combine_errors<const N: usize>(results: [Result<()>; N]) -> Result<()> {
    let errors: Vec<_> = results.into_iter().filter_map(Result::err).collect();
    if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
}
fn request(command: AudioCommand) -> Result<()> {
    use std::sync::{mpsc, OnceLock};
    static WORKER: OnceLock<mpsc::Sender<AudioRequest>> = OnceLock::new();
    let worker = WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<AudioRequest>();
        std::thread::spawn(move || {
            let mut audio = AlarmAudio::default();
            let mut shutting_down = false;
            for (command, reply) in rx {
                let result = match command {
                    AudioCommand::Alarm(active, percent, device, mute_others) if !shutting_down => audio.sync(active, percent, device.as_deref(), mute_others),
                    AudioCommand::Test(device) if !shutting_down => {
                        if audio.ringing { Err("Silence the active alarms before testing sound".into()) }
                        else { play_raw(false, device.as_deref()) }
                    }
                    AudioCommand::Shutdown => { shutting_down = true; stop_raw(); audio.stop() }
                    _ => Ok(()),
                };
                let _ = reply.send(result);
            }
        });
        tx
    });
    let (tx, rx) = mpsc::channel();
    worker.send((command, tx)).map_err(|_| "Audio worker stopped".to_string())?;
    rx.recv().map_err(|_| "Audio worker stopped".to_string())?
}
pub fn sync_alarm(active: bool, percent: Option<u8>, device: Option<String>, mute_others: bool) -> Result<()> {
    request(AudioCommand::Alarm(active, percent, device, mute_others))
}
pub fn test_sound(device: Option<String>) -> Result<()> { request(AudioCommand::Test(device)) }
pub fn shutdown() -> Result<()> { request(AudioCommand::Shutdown) }

fn play_raw(repeat:bool, _device: Option<&str>) -> Result<()> {
    #[cfg(windows)] { crate::windows_audio::play(SOUND, _device, repeat)?; }
    #[cfg(target_os="macos")] {
        let _repeat=repeat;
        let file=std::env::temp_dir().join(format!("timebridge-{}-alert.wav",std::process::id()));
        std::fs::write(&file,SOUND).map_err(|e|e.to_string())?;
        MAC_STOP.store(false,std::sync::atomic::Ordering::SeqCst);
        std::thread::spawn(move || {
            let Ok(mut child)=std::process::Command::new("/usr/bin/afplay").arg(&file).spawn() else {let _=std::fs::remove_file(file);return;};
            while !child.try_wait().ok().flatten().is_some() {
                if MAC_STOP.load(std::sync::atomic::Ordering::SeqCst) {let _=child.kill();let _=child.wait();break;}
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            let _=std::fs::remove_file(file);
        });
    }
    #[cfg(target_os="linux")] {
        use std::process::{Command,Stdio};
        let _repeat=repeat;
        stop_raw();
        let mut child=Command::new("paplay").stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
            .or_else(|_|Command::new("aplay").args(["-q"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn())
            .map_err(|_|"Install paplay or aplay to play Timebridge alerts".to_string())?;
        if let Some(mut input)=child.stdin.take(){std::io::Write::write_all(&mut input,SOUND).map_err(|e|e.to_string())?;}
        *LINUX_CHILD.lock().map_err(|_|"Audio lock failed")?=Some(child);
    }
    Ok(())
}
fn stop_raw() {
    #[cfg(windows)] { crate::windows_audio::stop(); }
    #[cfg(target_os="macos")] { MAC_STOP.store(true,std::sync::atomic::Ordering::SeqCst); }
    #[cfg(target_os="linux")] {
        if let Ok(mut child)=LINUX_CHILD.lock(){if let Some(mut child)=child.take(){let _=child.kill();let _=child.wait();}}
    }
}

#[cfg(target_os="macos")]
static MAC_STOP:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
#[cfg(target_os="linux")]
static LINUX_CHILD:std::sync::Mutex<Option<std::process::Child>>=std::sync::Mutex::new(None);
