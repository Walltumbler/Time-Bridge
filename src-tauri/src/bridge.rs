//! Loopback v1 bridge. Origin comes from HTTP, never from a site-supplied item.
use crate::{model::*, store::{Store, Event}};
use axum::{body::Bytes, extract::{DefaultBodyLimit, State}, http::{HeaderMap, Method, StatusCode}, response::{IntoResponse, Response}, routing::any, Json, Router};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::{Arc, Mutex}};
use subtle::ConstantTimeEq;

pub const PORT: u16 = 47832;
pub const MAX_BODY: usize = 32768;
fn hash(token:&str)->String { format!("{:x}",Sha256::digest(token.as_bytes())) }
fn secret()->String { format!("{}{}",uuid::Uuid::new_v4().simple(),uuid::Uuid::new_v4().simple()) }
fn matches_secret(a:&str,b:&str)->bool { a.as_bytes().ct_eq(b.as_bytes()).into() }
fn err(e:impl std::fmt::Display)->String {e.to_string()}

#[derive(Debug)]
pub struct ApiError(pub u16,pub String);
type ApiResult<T> = std::result::Result<T,ApiError>;
impl From<String> for ApiError {fn from(s:String)->Self {Self(400,s)}}
impl From<&str> for ApiError {fn from(s:&str)->Self {Self(400,s.into())}}
impl IntoResponse for ApiError {fn into_response(self)->Response { (StatusCode::from_u16(self.0).unwrap_or(StatusCode::BAD_REQUEST),Json(json!({"error":self.1}))).into_response() }}

#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Site {pub origin:String,pub name:String,pub approved_at:String}
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct PairView {pub id:String,pub origin:String,pub name:String,pub expires_at:DateTime<Utc>,pub status:String}
struct Pair { view:PairView, poll_hash:String, credential:Option<String> }
#[derive(Deserialize)]
#[serde(tag="op", deny_unknown_fields)]
pub enum Rpc {
    #[serde(rename="detect")] Detect,
    #[serde(rename="permissions.request")] Pair { name:String, #[serde(rename="pollSecret")] poll_secret:String },
    #[serde(rename="permissions.poll")] Poll { id:String, #[serde(rename="pollSecret")] poll_secret:String },
    #[serde(rename="permissions.status")] PermissionStatus,
    #[serde(rename="items.list")] List,
    #[serde(rename="item.get")] Get {id:String},
    #[serde(rename="item.create")] Create {input:CreateItem},
    #[serde(rename="item.action")] Action {id:String,action:WebAction},
    #[serde(rename="item.update")] Update {id:String,title:String},
    #[serde(rename="events.listUndelivered")] Events,
    #[serde(rename="events.acknowledge")] Ack {ids:Vec<i64>},
}
#[derive(Deserialize)]
#[serde(rename_all="snake_case")]
pub enum WebAction {Pause,Resume,Cancel,Done,Dismiss,Silence,Extend1,Extend5}
impl WebAction { fn name(&self)->&str {match self {Self::Pause=>"pause",Self::Resume=>"resume",Self::Cancel=>"cancel",Self::Done=>"done",Self::Dismiss=>"dismiss",Self::Silence=>"silence",Self::Extend1=>"extend1",Self::Extend5=>"extend5"}} }

pub struct Bridge {
    pub store:Arc<Mutex<Store>>,
    pairs:Mutex<HashMap<String,Pair>>,
    rates:Mutex<HashMap<String,(DateTime<Utc>,u32)>>,
}
impl Bridge {
    pub fn new(store:Arc<Mutex<Store>>)->Self { Self {store,pairs:Mutex::new(HashMap::new()),rates:Mutex::new(HashMap::new())} }
    pub fn pending(&self)->Result<Vec<PairView>> {
        let mut p=self.pairs.lock().map_err(err)?;
        p.retain(|_,p|p.view.expires_at>Utc::now());
        Ok(p.values().filter(|p|p.view.status=="pending").map(|p|p.view.clone()).collect())
    }
    pub fn approve(&self,id:&str,allow:bool)->Result<()> {
        let mut pairs=self.pairs.lock().map_err(err)?;
        let p=pairs.get_mut(id).filter(|p|p.view.expires_at>Utc::now()&&p.view.status=="pending").ok_or("Pairing request expired or already answered")?;
        if allow {
            let credential=secret();
            self.store.lock().map_err(err)?.conn.execute("INSERT INTO site_permissions(origin,name,credential_hash,approved_at,revoked) VALUES(?1,?2,?3,?4,0) ON CONFLICT(origin) DO UPDATE SET name=excluded.name,credential_hash=excluded.credential_hash,approved_at=excluded.approved_at,revoked=0",params![p.view.origin,p.view.name,hash(&credential),Utc::now().to_rfc3339()]).map_err(err)?;
            p.credential=Some(credential);p.view.status="approved".into();
        } else {p.view.status="denied".into();}
        Ok(())
    }
    pub fn sites(&self)->Result<Vec<Site>> {
        let store=self.store.lock().map_err(err)?;
        let mut stmt=store.conn.prepare("SELECT origin,name,approved_at FROM site_permissions WHERE revoked=0 ORDER BY origin").map_err(err)?;
        let rows=stmt.query_map([],|r|Ok(Site{origin:r.get(0)?,name:r.get(1)?,approved_at:r.get(2)?})).map_err(err)?;
        rows.map(|r|r.map_err(err)).collect()
    }
    pub fn revoke(&self,origin:&str)->Result<()> {
        // Same lock order as approval; invalidate unclaimed credentials as well.
        self.pairs.lock().map_err(err)?.retain(|_,p|p.view.origin!=origin);
        self.store.lock().map_err(err)?.conn.execute("UPDATE site_permissions SET revoked=1,credential_hash='' WHERE origin=?1",[origin]).map_err(err)?;
        Ok(())
    }
    fn rate(&self,key:&str,max:u32,now:DateTime<Utc>)->ApiResult<()> {
        let mut rates=self.rates.lock().map_err(|_|ApiError(503,"Service unavailable".into()))?;
        rates.retain(|_,(start,_)|*start+Duration::minutes(1)>now);
        if rates.len()>=256 && !rates.contains_key(key) {return Err(ApiError(429,"Too many origins; retry later".into()));}
        let count=rates.entry(key.into()).or_insert((now,0));count.1+=1;
        if count.1>max {return Err(ApiError(429,"Rate limit reached; retry in a minute".into()));} Ok(())
    }
    pub fn rpc(&self,origin:&str,credential:Option<&str>,rpc:Rpc)->ApiResult<Value> {
        validate_origin(origin)?;
        let now=Utc::now(); self.rate("global",600,now)?;self.rate(origin,120,now)?;
        match rpc {
            Rpc::Detect => return Ok(json!({"desktop":true,"version":"0.2.0","protocol":1,"capabilities":["countdown","ranged_countdown","alarm","recurring_alarm","stopwatch"]})),
            Rpc::Pair{name,poll_secret} => {
                if name.trim().is_empty()||name.len()>120||poll_secret.len()!=64||!poll_secret.bytes().all(|b|b.is_ascii_hexdigit()) {return Err("Invalid pairing name or secret".into());}
                self.rate(&format!("pair:{origin}"),3,now)?;self.rate("pair:global",12,now)?;
                let mut pairs=self.pairs.lock().map_err(err)?;
                pairs.retain(|_,p|p.view.expires_at>now);
                if pairs.len()>=16 {return Err(ApiError(429,"Too many pairing requests".into()));}
                let id=uuid::Uuid::new_v4().to_string();let expires_at=now+Duration::minutes(2);
                pairs.insert(id.clone(),Pair{view:PairView{id:id.clone(),origin:origin.into(),name:name.trim().into(),expires_at,status:"pending".into()},poll_hash:hash(&poll_secret),credential:None});
                return Ok(json!({"id":id,"expiresAt":expires_at,"status":"pending"}));
            }
            Rpc::Poll{id,poll_secret} => {
                let mut pairs=self.pairs.lock().map_err(err)?;
                pairs.retain(|_,p|p.view.expires_at>now);
                let p=pairs.get(&id).filter(|p|p.view.origin==origin&&matches_secret(&p.poll_hash,&hash(&poll_secret))).ok_or(ApiError(404,"Pairing request not found or expired".into()))?;
                let result=json!({"status":p.view.status,"credential":p.credential});
                if p.view.status!="pending" {pairs.remove(&id);}
                return Ok(result);
            }
            _=>{}
        }
        let mut store=self.store.lock().map_err(err)?;
        let approved:Option<(String,String)>=store.conn.query_row("SELECT name,credential_hash FROM site_permissions WHERE origin=?1 AND revoked=0",[origin],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(err)?;
        let (name,expected)=approved.ok_or(ApiError(401,"Site is not paired".into()))?;
        if !matches_secret(&expected,&hash(credential.filter(|c|c.len()==64).ok_or(ApiError(401,"Pairing credential required".into()))?)) {return Err(ApiError(401,"Invalid pairing credential".into()));}
        let owned=|id:&str,store:&Store|->ApiResult<Item>{store.list()?.into_iter().find(|i|i.id==id&&i.source.as_ref().is_some_and(|s|s.origin==origin)).ok_or(ApiError(404,"Item not found".into()))};
        match rpc {
            Rpc::PermissionStatus=>Ok(json!({"approved":true,"origin":origin,"name":name})),
            Rpc::List=>Ok(json!(store.list()?.into_iter().filter(|i|i.source.as_ref().is_some_and(|s|s.origin==origin)).collect::<Vec<_>>())),
            Rpc::Get{id}=>Ok(json!(owned(&id,&store)?)),
            Rpc::Create{mut input}=>{
                if input.source.is_some() {return Err("Source is assigned by Timebridge; do not supply source metadata".into());}
                let items=store.list()?;
                if items.iter().filter(|i|i.active()&&i.source.as_ref().is_some_and(|s|s.origin==origin)).count()>=50 || items.iter().filter(|i|i.active()).count()>=500 {return Err(ApiError(429,"Active item limit reached".into()));}
                if let Some(external)=&input.external_id {
                    if let Some(existing)=items.iter().find(|i|i.external_id.as_ref()==Some(external)&&i.source.as_ref().is_some_and(|s|s.origin==origin)) {return Ok(json!(existing));}
                }
                input.source=Some(Source{name,origin:origin.into()});
                Ok(json!(store.create_as(input,now,"website")?))
            }
            Rpc::Action{id,action}=>{owned(&id,&store)?;store.action_as(&id,action.name(),now,"website")?;Ok(json!(owned(&id,&store)?))}
            Rpc::Update{id,title}=>{owned(&id,&store)?;store.rename(&id,&title,now)?;Ok(json!(owned(&id,&store)?))}
            Rpc::Events=>{
                let mut stmt=store.conn.prepare("SELECT e.id,e.payload FROM events e WHERE json_extract(e.payload,'$.origin')=?1 AND NOT EXISTS(SELECT 1 FROM event_acks a WHERE a.origin=?1 AND a.event_id=e.id) ORDER BY e.id LIMIT 100").map_err(err)?;
                let rows=stmt.query_map([origin],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).map_err(err)?;
                let mut result=vec![];for row in rows {let (id,payload)=row.map_err(err)?;let mut event:Event=serde_json::from_str(&payload).map_err(err)?;event.id=id;result.push(event);}
                Ok(json!(result))
            }
            Rpc::Ack{ids}=>{
                if ids.len()>100||ids.iter().any(|id|*id<=0) {return Err("At most 100 positive event IDs are allowed".into());}
                let tx=store.conn.transaction().map_err(err)?;
                for id in ids {tx.execute("INSERT OR IGNORE INTO event_acks(origin,event_id) SELECT ?1,id FROM events WHERE id=?2 AND json_extract(payload,'$.origin')=?1",params![origin,id]).map_err(err)?;}
                tx.commit().map_err(err)?;Ok(json!({"ok":true}))
            }
            _=>unreachable!(),
        }
    }
}

pub fn router(bridge:Arc<Bridge>)->Router {
    Router::new().route("/v1",any(http)).layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(tower::limit::ConcurrencyLimitLayer::new(32))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT,std::time::Duration::from_secs(5)))
        .with_state(bridge).merge(crate::setup::router())
}
async fn http(State(bridge):State<Arc<Bridge>>,method:Method,headers:HeaderMap,body:Bytes)->Response {
    let one=|key:&str|->Option<&str>{if headers.get_all(key).iter().count()!=1 {None}else{headers.get(key)?.to_str().ok()}};
    if one("host")!=Some("127.0.0.1:47832") {return ApiError(403,"Invalid Host".into()).into_response();}
    let Some(origin)=one("origin").filter(|o|validate_origin(o).is_ok()) else {return ApiError(403,"Valid Origin required".into()).into_response();};
    let mut response=if method==Method::OPTIONS {(StatusCode::NO_CONTENT,"").into_response()}
        else if method!=Method::POST {ApiError(405,"Use POST".into()).into_response()}
        else if one("content-type").map(|v|v.split(';').next().unwrap_or("").trim())!=Some("application/json") {ApiError(415,"Use application/json".into()).into_response()}
        else {
            match serde_json::from_slice::<Rpc>(&body) {
                Ok(rpc)=>match bridge.rpc(origin,one("authorization").and_then(|v|v.strip_prefix("Bearer ")),rpc) {Ok(v)=>Json(v).into_response(),Err(e)=>e.into_response()},
                Err(_)=>ApiError(400,"Invalid v1 request schema".into()).into_response(),
            }
        };
    let h=response.headers_mut();
    h.insert("Access-Control-Allow-Origin",origin.parse().unwrap());h.insert("Vary","Origin".parse().unwrap());
    h.insert("Access-Control-Allow-Methods","POST, OPTIONS".parse().unwrap());h.insert("Access-Control-Allow-Headers","Content-Type, Authorization".parse().unwrap());
    h.insert("Access-Control-Allow-Private-Network","true".parse().unwrap());h.insert("Cache-Control","no-store".parse().unwrap());
    response
}
pub async fn serve(bridge:Arc<Bridge>)->Result<()> {
    let listener=tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,PORT)).await.map_err(err)?;
    axum::serve(listener,router(bridge)).await.map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::Path, sync::{Arc, Mutex}};
    #[tokio::test]
    async fn http_client_can_pair_create_icon_alarm_and_read_own_items() {
        use tower::ServiceExt;
        use axum::{body::{Body, to_bytes}, http::Request};
        use base64::Engine;
        let bridge = Arc::new(bridge());
        let app = router(bridge.clone());
        async fn send(app: Router, body: Value, credential: Option<&str>) -> (StatusCode, Value) {
            let mut request = Request::builder().method("POST").uri("/v1")
                .header("Host", "127.0.0.1:47832").header("Origin", "http://127.0.0.1:4790")
                .header("Content-Type", "application/json");
            if let Some(credential) = credential { request = request.header("Authorization", format!("Bearer {credential}")); }
            let response = app.oneshot(request.body(Body::from(body.to_string())).unwrap()).await.unwrap();
            let status = response.status();
            let body = serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap()).unwrap();
            (status, body)
        }
        let (status, _) = send(app.clone(), json!({"op":"items.list"}), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let secret = "a".repeat(64);
        let (_, pair) = send(app.clone(), json!({"op":"permissions.request","name":"Example demo","pollSecret":secret}), None).await;
        let id = pair["id"].as_str().unwrap();
        bridge.approve(id, true).unwrap(); // Test-only in-memory desktop decision.
        let (_, approved) = send(app.clone(), json!({"op":"permissions.poll","id":id,"pollSecret":secret}), None).await;
        let credential = approved["credential"].as_str().unwrap();
        let icon = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(include_bytes!("../icons/32x32.png")));
        let (status, created) = send(app.clone(), json!({"op":"item.create","input":{"kind":"countdown","durationSeconds":10,"icon":icon,"returnUrl":"http://127.0.0.1:4790/"}}), Some(credential)).await;
        assert_eq!(status, StatusCode::OK, "{created}");
        assert_eq!(created["title"], "10 second timer");
        assert_eq!(created["icon"], icon);
        assert_eq!(created["source"]["origin"], "http://127.0.0.1:4790");
        let (_, items) = send(app, json!({"op":"items.list"}), Some(credential)).await;
        assert_eq!(items.as_array().unwrap().len(), 1);
    }

    fn bridge() -> Bridge {
        let store = Store::open(Path::new(":memory:")).unwrap();
        Bridge::new(Arc::new(Mutex::new(store)))
    }
    fn rpc(bridge:&Bridge, origin:&str, token:Option<&str>, body:Value)->ApiResult<Value> {
        bridge.rpc(origin, token, serde_json::from_value(body).unwrap())
    }
    fn pair(bridge:&Bridge, origin:&str)->String {
        let secret=uuid::Uuid::new_v4().simple().to_string().repeat(2);
        let request=rpc(bridge,origin,None,json!({"op":"permissions.request","name":"Example site","pollSecret":secret})).unwrap();
        let id=request["id"].as_str().unwrap().to_owned();
        let pending=bridge.pending().unwrap().into_iter().find(|p|p.id==id).unwrap();
        assert_eq!(pending.origin,origin);
        bridge.approve(&id,true).unwrap();
        let approved=rpc(bridge,origin,None,json!({"op":"permissions.poll","id":id,"pollSecret":secret})).unwrap();
        approved["credential"].as_str().unwrap().to_owned()
    }

    #[test]
    fn website_credentials_scope_items_events_and_revocation_by_exact_origin() {
        let bridge=bridge();
        let a="https://example.com";let b="https://other.example";
        let token_a=pair(&bridge,a);let token_b=pair(&bridge,b);
        assert_eq!(rpc(&bridge,a,None,json!({"op":"items.list"})).unwrap_err().0,401);
        let item=rpc(&bridge,a,Some(&token_a),json!({"op":"item.create","input":{"title":"","kind":"countdown","durationSeconds":60,"externalId":"work","returnUrl":"https://example.com/timer"}})).unwrap();
        let id=item["id"].as_str().unwrap().to_owned();
        assert_eq!(item["title"],"1 minute timer");assert_eq!(item["source"]["origin"],a);
        assert_eq!(rpc(&bridge,b,Some(&token_b),json!({"op":"items.list"})).unwrap().as_array().unwrap().len(),0);
        assert_eq!(rpc(&bridge,b,Some(&token_b),json!({"op":"item.action","id":id,"action":"cancel"})).unwrap_err().0,404);
        let events=rpc(&bridge,a,Some(&token_a),json!({"op":"events.listUndelivered"})).unwrap();
        assert_eq!(events.as_array().unwrap().len(),1);
        let event_id=events[0]["id"].as_i64().unwrap();
        rpc(&bridge,b,Some(&token_b),json!({"op":"events.acknowledge","ids":[event_id]})).unwrap();
        assert_eq!(rpc(&bridge,a,Some(&token_a),json!({"op":"events.listUndelivered"})).unwrap().as_array().unwrap().len(),1);
        rpc(&bridge,a,Some(&token_a),json!({"op":"events.acknowledge","ids":[event_id]})).unwrap();
        assert!(rpc(&bridge,a,Some(&token_a),json!({"op":"events.listUndelivered"})).unwrap().as_array().unwrap().is_empty());
        bridge.revoke(a).unwrap();
        assert_eq!(rpc(&bridge,a,Some(&token_a),json!({"op":"items.list"})).unwrap_err().0,401);
        assert_eq!(bridge.store.lock().unwrap().list().unwrap().len(),1);
    }

    #[test]
    fn create_rejects_a_return_url_from_another_site() {
        let bridge=bridge();let origin="https://example.com";let token=pair(&bridge,origin);
        let result=rpc(&bridge,origin,Some(&token),json!({"op":"item.create","input":{"kind":"countdown","durationSeconds":60,"returnUrl":"https://attacker.example/"}}));
        assert_eq!(result.unwrap_err().0,400);
    }

    #[test]
    fn pairing_poll_and_origin_validation_are_bound_to_request_origin() {
        let bridge=bridge();let origin="https://example.com";
        let secret=uuid::Uuid::new_v4().simple().to_string().repeat(2);
        let pair=rpc(&bridge,origin,None,json!({"op":"permissions.request","name":"Example","pollSecret":secret})).unwrap();
        let id=pair["id"].as_str().unwrap();
        assert_eq!(rpc(&bridge,"https://attacker.example",None,json!({"op":"permissions.poll","id":id,"pollSecret":secret})).unwrap_err().0,404);
        assert!(rpc(&bridge,"https://example.com.evil",None,json!({"op":"detect"})).is_ok());
        assert_eq!(bridge.pending().unwrap()[0].origin,origin);
        assert!(validate_origin("https://example.com/path").is_err());
        assert!(validate_origin("http://192.168.1.4").is_err());
        assert!(validate_origin("http://localhost:5173").is_ok());
    }
}
