#![no_std]
mod protocol;
use aidoku::{Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, HashMap, ImageRequestProvider, Listing, ListingProvider, Manga, MangaPageResult, MangaStatus, NotificationHandler, Page, PageContent, Result, Source, WebLoginHandler, alloc::{String, Vec, vec, format, string::ToString}, helpers::uri::encode_uri, imports::{defaults::{DefaultValue, defaults_get, defaults_set}, net::Request, std::current_date}, prelude::*};
use serde_json::{Value, json};
use hmac::{Hmac, Mac};
use sha2::Sha256;

struct BikaWeb;
mod blocking;
use aidoku::{DynamicListings, DynamicFilters, Filter, SelectFilter, SortFilter, SortFilterDefault};

fn categories() -> Result<Vec<String>> {
    let response = api("categories", None)?;
    let rows = response["data"]["categories"].as_array()
        .ok_or_else(|| error!("哔咔分类数据缺失，请登录后刷新图源。"))?;
    let mut names = Vec::new();
    for row in rows {
        let name = text(row, "title").trim();
        // Website-only links and inactive tiles are not comic categories.
        if name.is_empty() || row["isWeb"].as_bool() == Some(true)
            || row["active"].as_bool() == Some(false)
            || !text(row, "link").is_empty() { continue; }
        if !names.iter().any(|s| s == name) { names.push(name.to_string()); }
    }
    if names.is_empty() { bail!("哔咔没有返回可用分类，请登录后刷新图源。"); }
    Ok(names)
}

impl DynamicListings for BikaWeb {
    fn get_dynamic_listings(&self) -> Result<Vec<Listing>> {
        let mut listings = vec![
            Listing { id: "latest".into(), name: "最近更新".into(), ..Default::default() },
            Listing { id: "popular".into(), name: "热门漫画".into(), ..Default::default() },
            Listing { id: "rank:H24".into(), name: "排行榜 · 24小时".into(), ..Default::default() },
            Listing { id: "rank:D7".into(), name: "排行榜 · 7天".into(), ..Default::default() },
            Listing { id: "rank:D30".into(), name: "排行榜 · 30天".into(), ..Default::default() },
        ];
        listings.extend(categories()?.into_iter().map(|name| Listing {
            id: format!("category:{name}"), name, ..Default::default()
        }));
        Ok(listings)
    }
}
impl DynamicFilters for BikaWeb {
    fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
        let mut options = vec!["全部分类".into()];
        let mut ids = vec!["".into()];
        for name in categories()? { options.push(name.clone().into()); ids.push(name.into()); }
        Ok(vec![SelectFilter {
            id: "category".into(), title: Some("分类".into()), options,
            ids: Some(ids), default: Some("".into()), ..Default::default()
        }.into(), SortFilter {
            id: "sort".into(), title: Some("排序".into()), can_ascend: false,
            options: vec!["最新".into(), "最早".into(), "最多喜欢".into(), "最多浏览".into()],
            default: Some(SortFilterDefault { index: 0, ascending: false }),
            ..Default::default()
        }.into()])
    }
}
const SITE: &str = "https://manhuabika.com";
fn text<'a>(v: &'a Value, k: &str) -> &'a str { v.get(k).and_then(Value::as_str).unwrap_or("") }
fn route() -> String {
    match defaults_get::<String>("route").as_deref() { Some("acbbb.com") => "acbbb.com".into(), _ => "go2778.com".into() }
}
fn signature(path: &str, time: &str, nonce: &str, method: &str) -> String {
    let msg = format!("{path}{time}{nonce}{method}{}", protocol::API_KEY).to_ascii_lowercase();
    let mut mac = Hmac::<Sha256>::new_from_slice(protocol::SIGNING_KEY.as_bytes()).unwrap();
    mac.update(msg.as_bytes());
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}
fn api(path: &str, body: Option<Value>) -> Result<Value> {
    let url = format!("https://picaapi.{}/{path}", route());
    let method = if body.is_some() { "POST" } else { "GET" };
    let time = current_date().to_string();
    // Nonce is a request identifier; use a deterministic 32-character HMAC value.
    let nonce = signature(path, &time, "nonce", method)[..32].to_string();
    let sign = signature(path, &time, &nonce, method);
    let mut req = if body.is_some() { Request::post(url)? } else { Request::get(url)? };
    for (k,v) in [("app-channel","1"),("app-uuid","webUUIDv2"),("app-version","20251017"),("accept","application/vnd.picacomic.com.v1+json"),("app-platform","android"),("Content-Type","application/json; charset=UTF-8"),("time",time.as_str()),("nonce",nonce.as_str()),("signature",sign.as_str())] { req.set_header(k,v); }
    let quality = defaults_get::<String>("imageQuality").unwrap_or("medium".into());
    req.set_header("image-quality",quality.as_str());
    if let Some(token) = defaults_get::<String>("login.ls.token").filter(|s| !s.is_empty()) { req.set_header("authorization",token.as_str()); }
    if let Some(body) = body { req.set_body(body.to_string()); }
    let value: Value = req.json_owned()?;
    match value.get("code").and_then(Value::as_i64) {
        Some(200) => Ok(value),
        Some(401) => bail!("哔咔登录已失效，请在图源设置重新网页登录。"),
        _ => bail!("哔咔接口请求失败：{}", text(&value,"message")),
    }
}
fn media(v: &Value) -> Result<String> {
    let server = text(v,"fileServer"); let path = text(v,"path");
    if !(server.starts_with("https://") || server.starts_with("http://")) || path.is_empty() { bail!("哔咔图片地址缺失"); }
    let server = server.trim_end_matches('/');
    let url = format!("{server}{}/{}", if server.ends_with("/static") { "" } else { "/static" }, path.trim_start_matches('/'));
    if defaults_get::<String>("imageRoute").as_deref() == Some("official") { return Ok(url); }
    Ok(url.replace("manhuabika.com", &route()).replace("diwodiwo.xyz", &format!("storage1.{}",route())).replace("picacomic.com", &route()))
}
fn manga(v: &Value) -> Result<Manga> {
    let id = text(v,"_id"); let title = text(v,"title");
    if id.is_empty() || title.is_empty() { bail!("哔咔漫画资料不完整"); }
    let mut tags = Vec::new();
    for field in ["categories","tags"] { if let Some(a)=v[field].as_array() { for t in a { if let Some(s)=t.as_str() { if !tags.iter().any(|t| t==s) { tags.push(s.to_string()); } } } } }
    Ok(Manga {key:id.into(),title:title.into(),cover:Some(media(&v["thumb"])?),url:Some(format!("{SITE}/comic/{id}")),description:Some(text(v,"description").into()),authors:if text(v,"author").is_empty() {None} else {Some(vec![text(v,"author").into()])},tags:Some(tags),status:if v["finished"].as_bool().unwrap_or(false) {MangaStatus::Completed} else {MangaStatus::Ongoing},content_rating:ContentRating::NSFW,..Default::default()})
}
fn docs<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>> { v["data"][key]["docs"].as_array().ok_or_else(||error!("哔咔分页数据缺失")) }
fn more(v: &Value, key: &str, page:i32) -> Result<bool> {
    let pages=v["data"][key]["pages"].as_i64().ok_or_else(||error!("哔咔分页页数缺失"))?;
    if pages>10000 { bail!("哔咔分页页数异常"); }
    Ok((page as i64)<pages)
}
fn list(page:i32, sort:&str, category:&str, query:Option<&str>) -> Result<MangaPageResult> {
    let (path,body)=if let Some(q)=query { let mut body=json!({"keyword":q,"sort":sort}); if !category.is_empty() {body["categories"]=json!([category]);} (format!("comics/advanced-search?page={page}&s={sort}"),Some(body)) } else { (format!("comics?page={page}&s={sort}{}",if category.is_empty(){String::new()}else{format!("&c={}",encode_uri(category))}),None) };
    let v=api(&path,body)?;
    Ok(MangaPageResult{entries:blocking::visible(docs(&v,"comics")?)?,has_next_page:more(&v,"comics",page)?})
}
impl Source for BikaWeb {
    fn new()->Self {Self}
    fn get_search_manga_list(&self, query:Option<String>, page:i32, filters:Vec<FilterValue>)->Result<MangaPageResult> {
        let mut category=String::new();let mut sort="dd";
        for f in filters {match f { FilterValue::Text{id,value} | FilterValue::Select{id,value} if id=="category"=>category=value,FilterValue::Sort{id,index,..} if id=="sort"=>sort=*(["dd","da","ld","vd"].get(index as usize).unwrap_or(&"dd")),_=>{}}}
        list(page.max(1),sort,&category,query.as_deref().filter(|s|!s.is_empty()))
    }
    fn get_manga_update(&self, mut m:Manga, details:bool, chapters:bool)->Result<Manga> {
        blocking::check_known(&m.key)?;
        if details || (chapters && blocking::enabled()) {let v=api(&format!("comics/{}",encode_uri(&m.key)),None)?;blocking::check(&v["data"]["comic"])?;m.copy_from(manga(&v["data"]["comic"])?);}
        if chapters {let mut all=Vec::new();let mut page=1;
            loop {let v=api(&format!("comics/{}/eps?page={page}",encode_uri(&m.key)),None)?;
                for e in docs(&v,"eps")? {let order=e["order"].as_i64().filter(|n|*n>0).ok_or_else(||error!("哔咔章节序号缺失"))?;
                    all.push(Chapter{key:order.to_string(),title:Some(text(e,"title").into()),chapter_number:Some(order as f32),url:Some(format!("{SITE}/comic/reader/{}/{order}",m.key)),..Default::default()});}
                if !more(&v,"eps",page)? {break;} page+=1;
            }
            all.sort_by(|a,b|b.chapter_number.partial_cmp(&a.chapter_number).unwrap_or(core::cmp::Ordering::Equal));
            all.dedup_by(|a,b|a.key==b.key);m.chapters=Some(all);
        } Ok(m)
    }
    fn get_page_list(&self,m:Manga,c:Chapter)->Result<Vec<Page>> {
        blocking::check_known(&m.key)?;
        if blocking::enabled() {let v=api(&format!("comics/{}",encode_uri(&m.key)),None)?;blocking::check(&v["data"]["comic"])?;}
        let order=c.key.parse::<u32>().ok().filter(|n|*n>0).ok_or_else(||error!("哔咔章节标识无效"))?;
        let mut all=Vec::new();let mut page=1;
        loop {let v=api(&format!("comics/{}/order/{order}/pages?page={page}",encode_uri(&m.key)),None)?;
            for p in docs(&v,"pages")? {all.push(Page{content:PageContent::url(media(&p["media"])?),..Default::default()});}
            if !more(&v,"pages",page)? {break;} page+=1;
        } if all.is_empty() {bail!("哔咔章节没有可读图片");} Ok(all)
    }
}
fn leaderboard(time: &str, page: i32) -> Result<MangaPageResult> {
    if !["H24", "D7", "D30"].contains(&time) { bail!("哔咔排行榜时间无效"); }
    // Website returns one complete array, not a paginated comics object.
    if page > 1 { return Ok(MangaPageResult::default()); }
    let response = api(&format!("comics/leaderboard?tt={time}&ct=VC"), None)?;
    let rows = response["data"]["comics"].as_array()
        .ok_or_else(|| error!("哔咔排行榜数据缺失"))?;
    Ok(MangaPageResult {
        entries: blocking::visible(rows)?,
        has_next_page: false,
    })
}
impl ListingProvider for BikaWeb {
    fn get_manga_list(&self, l: Listing, page: i32) -> Result<MangaPageResult> {
        if let Some(time) = l.id.strip_prefix("rank:") { return leaderboard(time, page); }
        list(page.max(1), if l.id=="popular" {"vd"} else {"dd"}, l.id.strip_prefix("category:").unwrap_or(""), None)
    }
}
impl ImageRequestProvider for BikaWeb {fn get_image_request(&self,url:String,_context:Option<aidoku::PageContext>)->Result<Request>{Ok(Request::get(url)? )}}
impl WebLoginHandler for BikaWeb {
    fn handle_web_login(&self,_key:String,values:HashMap<String,String>)->Result<bool>{
        if let Some(token)=values.get("token").cloned().or_else(||defaults_get::<String>("login.ls.token")).filter(|s|!s.is_empty()).as_ref() {defaults_set("login.ls.token",DefaultValue::String(token.clone()));Ok(true)}else{Ok(false)}
    }
}
impl NotificationHandler for BikaWeb {fn handle_notification(&self,n:String){if n=="login" && defaults_get::<String>("login").as_deref()!=Some("logged_in"){defaults_set("login.ls.token",DefaultValue::Null);}}}
impl DeepLinkHandler for BikaWeb {fn handle_deep_link(&self,url:String)->Result<Option<DeepLinkResult>>{
    let prefix=format!("{SITE}/comic/");if let Some(id)=url.strip_prefix(&prefix).filter(|s|!s.is_empty()&&!s.contains('/')) {Ok(Some(DeepLinkResult::Manga{key:id.into()}))}else{Ok(None)}
}}
register_source!(BikaWeb,ListingProvider,DynamicListings,DynamicFilters,ImageRequestProvider,WebLoginHandler,NotificationHandler,DeepLinkHandler);


