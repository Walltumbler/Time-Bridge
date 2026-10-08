#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::Utc;
use std::sync::{Arc,Mutex};
use tauri::{Emitter,Manager,State,UserAttentionType,WebviewUrl,WebviewWindowBuilder,menu::{Menu,MenuItem},tray::TrayIconBuilder};
use tauri_plugin_opener::OpenerExt;
use timebridge_core::{audio,bridge::{self,Bridge,PairView,Site},integration,model::*,store::{Store,Event,Settings}};

struct AppState {store:Arc<Mutex<Store>>,bridge:Arc<Bridge>,bridge_error:Arc<Mutex<Option<String>>>,helper_error:Option<String>}
fn with_store<T>(s:&AppState,f:impl FnOnce(&mut Store)->Result<T>)->Result<T>{let mut store=s.store.lock().map_err(|_|"Storage lock failed")?;f(&mut store)}
#[tauri::command]
fn list_items(s:State<AppState>)->Result<Vec<Item>>{with_store(&s,|s|s.list())}
#[tauri::command]
fn list_events(s:State<AppState>)->Result<Vec<Event>>{with_store(&s,|s|s.events())}
#[tauri::command]
fn create_item(s:State<AppState>,input:CreateItem)->Result<Item>{
    if input.source.is_some()||input.return_url.is_some()||input.external_id.is_some(){return Err("Local items cannot claim a website source".into());}
    with_store(&s,|s|s.create(input,Utc::now()))
}
#[tauri::command]
fn item_action(s:State<AppState>,id:String,action:String)->Result<()>{with_store(&s,|s|s.action(&id,&action,Utc::now()))}
#[tauri::command]
fn rename_item(s:State<AppState>,id:String,title:String)->Result<()>{with_store(&s,|s|s.rename(&id,&title,Utc::now()))}
#[tauri::command]
fn snooze_item(s:State<AppState>,id:String)->Result<Item>{with_store(&s,|s|s.snooze(&id,Utc::now()))}
#[tauri::command]
fn get_settings(s:State<AppState>)->Result<Settings>{with_store(&s,|s|s.settings())}
#[tauri::command]
fn set_settings(s:State<AppState>,settings:Settings)->Result<()>{with_store(&s,|s|s.set_settings(settings))}
#[tauri::command]
fn clear_history(s:State<AppState>)->Result<()>{with_store(&s,|s|s.clear_history())}
#[tauri::command]
fn audio_status(s:State<AppState>)->Result<audio::AudioStatus>{Ok(audio::status(with_store(&s,|s|s.settings())?.alarm_output_device))}
#[tauri::command]
fn test_sound(device:Option<String>)->Result<()>{audio::test_sound(device)}
#[tauri::command]
fn audio_devices()->Vec<audio::AudioDevice>{audio::devices()}
#[tauri::command]
fn pairing_requests(s:State<AppState>)->Result<Vec<PairView>>{s.bridge.pending()}
#[tauri::command]
fn approve_site(s:State<AppState>,id:String,allow:bool)->Result<()>{s.bridge.approve(&id,allow)}
#[tauri::command]
fn approved_sites(s:State<AppState>)->Result<Vec<Site>>{s.bridge.sites()}
#[tauri::command]
fn revoke_site(s:State<AppState>,origin:String)->Result<()>{s.bridge.revoke(&origin)}
#[tauri::command]
fn bridge_status(s:State<AppState>)->serde_json::Value{serde_json::json!({"error":s.bridge_error.lock().ok().and_then(|v|v.clone()),"helperError":s.helper_error,"port":bridge::PORT})}
#[tauri::command]
fn open_main(app:tauri::AppHandle){show(&app)}
#[tauri::command]
fn open_setup(app:tauri::AppHandle)->Result<()>{app.opener().open_url("http://127.0.0.1:47832/setup/",None::<&str>).map_err(|e|e.to_string())}
#[tauri::command]
fn open_source(app:tauri::AppHandle,s:State<AppState>,id:String)->Result<()>{
    let item=with_store(&s,|s|s.list())?.into_iter().find(|i|i.id==id).ok_or("Item not found")?;
    validate_return_url(item.source.as_ref(),item.return_url.as_deref())?;
    app.opener().open_url(item.return_url.ok_or("No return URL")?,None::<&str>).map_err(|e|e.to_string())
}
fn show(app:&tauri::AppHandle){if let Some(w)=app.get_webview_window("main"){let _=w.show();let _=w.unminimize();let _=w.set_focus();}}
fn notify(app:&tauri::AppHandle,event:&Event)->Result<()>{
    let age=Utc::now().signed_duration_since(event.occurred_at).num_minutes();
    let label=if event.event=="timer.minimum_reached"{"Ready window has started"}else{"Time’s up"};
    let overdue=if age>0{format!(" · {age} min ago")}else{String::new()};
    let mut notification=notify_rust::Notification::new();
    notification.summary(&event.title).body(&format!("{label}{overdue}\n{}\nOpen Timebridge to snooze or dismiss.",event.origin.as_deref().unwrap_or("Created locally")));
    #[cfg(windows)]{let exe=std::env::current_exe().map_err(|e|e.to_string())?;let folder=exe.parent().ok_or("Missing executable directory")?;if !folder.ends_with("target/debug")&&!folder.ends_with("target/release"){notification.app_id(&app.config().identifier);}}
    #[cfg(target_os="macos")]{notify_rust::set_application(&app.config().identifier).map_err(|e|e.to_string())?;}
    notification.show().map_err(|e|e.to_string())?;Ok(())
}
fn scheduler(app:tauri::AppHandle){
    let mut previous_alerts=String::new();let mut previous_pairs=String::new();let mut previous_audio_error=None;
    let mut retry_at=std::time::Instant::now();
    loop{
        let state=app.state::<AppState>();
        let result=with_store(&state,|s|{let changed=s.tick(Utc::now())?;Ok((changed,s.list()?,s.settings()?,s.pending_notifications()?))});
        match result{
            Ok((changed,items,settings,events))=>{
                if changed{let _=app.emit("items-changed",());}
                let alerts:Vec<_>=items.iter().filter(|i|i.alert.is_some()).collect();
                let key=alerts.iter().map(|i|format!("{}:{}:{}",i.id,i.alert.as_deref().unwrap_or(""),i.fire_at.map(|d|d.to_rfc3339()).unwrap_or_default())).collect::<Vec<_>>().join("|");
                if key!=previous_alerts{
                    if let Some(w)=app.get_webview_window("alerts"){if alerts.is_empty(){let _=w.hide();}else{let _=w.show();}}
                    if let Some(w)=app.get_webview_window("main"){let _=w.request_user_attention(if alerts.is_empty(){None}else{Some(UserAttentionType::Critical)});}
                    let _=app.emit("items-changed",());previous_alerts=key;
                }
                let sounding=settings.sound&&alerts.iter().any(|i|!i.alert_silenced);
                let volume=settings.alarm_volume_override.then_some(settings.alarm_volume_percent);
                let audio_error=audio::sync_alarm(sounding,volume,settings.alarm_output_device.clone(),settings.mute_other_apps).err();
                if audio_error!=previous_audio_error {if let Some(e)=&audio_error{let _=app.emit("service-error",e);}previous_audio_error=audio_error;}
                if std::time::Instant::now()>=retry_at{
                    for event in events{
                        if settings.notifications{if let Err(e)=notify(&app,&event){retry_at=std::time::Instant::now()+std::time::Duration::from_secs(30);let _=app.emit("service-error",format!("System notification unavailable; Timebridge alert is still active. {e}"));break;}}
                        let _=with_store(&state,|s|s.mark_notified(event.id));
                    }
                }
                if let Ok(pairs)=state.bridge.pending(){let mut ids=pairs.iter().map(|p|p.id.as_str()).collect::<Vec<_>>();ids.sort();let key=ids.join("|");if key!=previous_pairs{let _=app.emit("pairing-changed",());if !pairs.is_empty(){if let Some(w)=app.get_webview_window("main"){let _=w.show();let _=w.request_user_attention(Some(UserAttentionType::Informational));}}previous_pairs=key;}}
            }
            Err(e)=>{eprintln!("Scheduler: {e}");let _=app.emit("service-error",e);}
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
fn main(){
    tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app,_,_|show(app))).plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![list_items,list_events,create_item,item_action,rename_item,snooze_item,get_settings,set_settings,clear_history,audio_status,audio_devices,test_sound,pairing_requests,approve_site,approved_sites,revoke_site,bridge_status,open_main,open_setup,open_source])
        .setup(|app|{
            let directory=app.path().app_data_dir()?;std::fs::create_dir_all(&directory)?;
            let store=Arc::new(Mutex::new(Store::open(&directory.join("timebridge.db")).map_err(std::io::Error::other)?));
            let bridge=Arc::new(Bridge::new(store.clone()));let bridge_error=Arc::new(Mutex::new(None));
            let helper_error=integration::register(&directory).err();
            match std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,bridge::PORT)){
                Ok(listener)=>{listener.set_nonblocking(true)?;let b=bridge.clone();let errors=bridge_error.clone();tauri::async_runtime::spawn(async move{let result=match tokio::net::TcpListener::from_std(listener){Ok(l)=>axum::serve(l,bridge::router(b)).await.map_err(|e|e.to_string()),Err(e)=>Err(e.to_string())};if let Err(e)=result{if let Ok(mut state)=errors.lock(){*state=Some(e);}}});}
                Err(e)=>{*bridge_error.lock().unwrap()=Some(format!("Website pairing is unavailable: {e}"));}
            }
            app.manage(AppState{store,bridge,bridge_error,helper_error});
            WebviewWindowBuilder::new(app,"alerts",WebviewUrl::App("index.html?alert=1".into())).title("Timebridge · Time’s up").inner_size(440.0,430.0).min_inner_size(360.0,300.0).always_on_top(true).focused(false).visible(false).build()?;
            let open=MenuItem::with_id(app,"open","Open Timebridge",true,None::<&str>)?;
            let new=MenuItem::with_id(app,"new","New timer…",true,None::<&str>)?;
            let quit=MenuItem::with_id(app,"quit","Quit Timebridge (stops alerts)",true,None::<&str>)?;
            let menu=Menu::with_items(app,&[&open,&new,&quit])?;
            TrayIconBuilder::with_id("timebridge").icon(app.default_window_icon().ok_or("Missing app icon")?.clone()).tooltip("Timebridge · Time that keeps going.").menu(&menu).show_menu_on_left_click(true)
                .on_menu_event(|app,e|match e.id.as_ref(){"quit"=>app.exit(0),"new"=>{show(app);let _=app.emit("new-item",());},_=>show(app)}).build(app)?;
            let handle=app.handle().clone();std::thread::spawn(move||scheduler(handle));Ok(())
        })
        .on_window_event(|window,event|{if let tauri::WindowEvent::CloseRequested{api,..}=event{api.prevent_close();if window.label()=="alerts"{let state=window.state::<AppState>();let _=with_store(&state,|s|{for i in s.list()?.into_iter().filter(|i|i.alert.is_some()){s.action(&i.id,"dismiss",Utc::now())?;}Ok(())});}let _=window.hide();}})
        .build(tauri::generate_context!()).expect("Timebridge failed to start")
        .run(|app,event| { if matches!(event,tauri::RunEvent::ExitRequested{..}|tauri::RunEvent::Exit) {
            if let Err(e)=audio::shutdown(){let _=app.emit("service-error",e);}
        }});
}
