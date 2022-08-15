use std::collections::HashMap;
use regex::Regex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    println!("{:#?}", resp);

    let html = reqwest::get("https://pzzqz.com/")
        .await?;

    let text = html.text().await?;


    // println!("{:#?}", text);
    let re = Regex::new(r"(?s)<tr>\n<td>(.*?)</td>\n<td>(\d+)</td>\n<td.*?span>\n(\D+)\n</td>\n<td.*?(\d+)</span>.*?</td>\n<td>(\S+)</td>\n<td.*?>(\S+)</td>.*?</tr>")?;

    for cap in re.captures_iter(text.as_str()) {
        println!("{:#?}", cap);
        println!("{:?}, {:?}, {:?}", &cap[1], &cap[2], &cap[5]);
    }

    Ok(())
}