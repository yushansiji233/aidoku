use super::*;
fn keywords() -> Vec<String> {
    defaults_get::<Vec<String>>("blockedMetadataKeywords").unwrap_or_default()
        .into_iter().map(|s|s.trim().to_lowercase()).filter(|s|!s.is_empty()).collect()
}
pub(super) fn enabled() -> bool { !keywords().is_empty() }
fn cached(words: &[String]) -> Vec<String> {
    defaults_get::<String>("blockedIdCache").and_then(|s|serde_json::from_str::<(Vec<String>,Vec<String>)>(&s).ok())
        .filter(|(saved,_)|saved==words).map(|(_,ids)|ids).unwrap_or_default()
}
fn remember(words:&[String],ids:&[String]) {
    if let Ok(s)=serde_json::to_string(&(words,ids)) {defaults_set("blockedIdCache",DefaultValue::String(s));}
}
fn matches(v:&Value,words:&[String]) -> bool {
    let id=text(v,"_id").to_lowercase();
    if words.iter().any(|w|*w==id){return true;}
    // Include the creator/translator names, category names and tags as well as main metadata.
    for key in ["title","author","description","translator","categories","tags"] {
        let value=&v[key];
        if let Some(s)=value.as_str() {if words.iter().any(|w|s.to_lowercase().contains(w.as_str())) {return true;}}
        if let Some(rows)=value.as_array() {for s in rows.iter().filter_map(Value::as_str) {if words.iter().any(|w|s.to_lowercase().contains(w.as_str())) {return true;}}}
    }
    words.iter().any(|w|text(&v["_creator"],"name").to_lowercase().contains(w.as_str()))
}
pub(super) fn visible(rows:&[Value]) -> Result<Vec<Manga>> {
    let words=keywords();let mut ids=cached(&words);let mut result=Vec::new();
    for row in rows {
        if !words.is_empty() && matches(row,&words) {
            let id=text(row,"_id");if !id.is_empty()&&!ids.iter().any(|s|s==id){ids.push(id.into());}
            continue;
        }
        result.push(manga(row)?);
    }
    if !words.is_empty(){remember(&words,&ids);}
    Ok(result)
}
pub(super) fn check_known(id:&str) -> Result<()> {
    let words=keywords();
    if words.iter().any(|w|w==&id.to_lowercase()) || cached(&words).iter().any(|s|s==id) {bail!("这部漫画已被你的屏蔽设置隐藏。请修改屏蔽列表后再试。");}
    Ok(())
}
pub(super) fn check(row:&Value) -> Result<()> {
    let words=keywords();
    if !words.is_empty() && matches(row,&words) {
        let mut ids=cached(&words);let id=text(row,"_id");if !id.is_empty()&&!ids.iter().any(|s|s==id){ids.push(id.into());}
        remember(&words,&ids);
        bail!("这部漫画已被你的屏蔽设置隐藏。请修改屏蔽列表后再试。");
    }
    Ok(())
}
