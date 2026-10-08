use crate::model::Result;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct Identity {chrome_id:String,firefox_id:String}
pub fn register(directory:&std::path::Path)->Result<()> {
    let identity:Identity=serde_json::from_str(include_str!("../extension-identity.json")).map_err(|e|e.to_string())?;
    let exe=std::env::current_exe().map_err(|e|e.to_string())?;
    let host=exe.parent().ok_or("Missing app directory")?.join(if cfg!(windows){"timebridge-host.exe"}else{"timebridge-host"});
    if !host.exists() {return Err("The browser helper is missing. Reinstall Timebridge to enable extensions.".into());}
    let base=serde_json::json!({"name":"app.timebridge.bridge","description":"Timebridge local timer bridge","path":host,"type":"stdio"});
    let mut chrome=base.clone();chrome["allowed_origins"]=serde_json::json!([format!("chrome-extension://{}/",identity.chrome_id)]);
    let mut firefox=base;firefox["allowed_extensions"]=serde_json::json!([identity.firefox_id]);
    #[cfg(windows)] {
        use winreg::{RegKey,enums::HKEY_CURRENT_USER};
        let dir=directory.join("browser-bridge");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
        for (browser,manifest) in [("Google\\Chrome",chrome),("Mozilla",firefox)] {
            let path=dir.join(if browser=="Mozilla"{"firefox.json"}else{"chrome.json"});
            std::fs::write(&path,serde_json::to_vec_pretty(&manifest).unwrap()).map_err(|e|e.to_string())?;
            let (key,_)=RegKey::predef(HKEY_CURRENT_USER).create_subkey(format!("Software\\{browser}\\NativeMessagingHosts\\app.timebridge.bridge")).map_err(|e|e.to_string())?;
            key.set_value("",&path.to_string_lossy().to_string()).map_err(|e|e.to_string())?;
        }
    }
    #[cfg(target_os="macos")] {
        let home=std::env::var_os("HOME").ok_or("Home directory unavailable")?;
        for (browser,manifest) in [("Google/Chrome",chrome),("Mozilla",firefox)] {
            let dir=std::path::PathBuf::from(&home).join("Library/Application Support").join(browser).join("NativeMessagingHosts");
            std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
            std::fs::write(dir.join("app.timebridge.bridge.json"),serde_json::to_vec_pretty(&manifest).unwrap()).map_err(|e|e.to_string())?;
        }
    }
    Ok(())
}
